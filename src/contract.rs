use cosmwasm_std::{
    entry_point, to_binary,   CosmosMsg, Deps, DepsMut,Binary,QueryRequest,WasmQuery,
    Env, MessageInfo, BankMsg, Response, StdResult, Uint128, WasmMsg, Coin, Order
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    State,CONFIG,TOKENINFO,TokenInfo
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw20::{Cw20ExecuteMsg,Cw20QueryMsg,BalanceResponse};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner:info.sender.to_string(),
        denom:msg.denom,
        staking_period : msg.staking_period,
        reward_wallet : msg.reward_wallet,
        total_staked : Uint128::new(0),
        nft_address : "nft_address".to_string(),
        token_address : "token_address".to_string(),
        can_stake: true,
        last_distribute:env.block.time.seconds()
    };
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(rcv_msg) => execute_stake_nft(deps, env, info, rcv_msg),
        ExecuteMsg::UnstakeNft { token_id } => execute_unstake_nft(deps, env, info, token_id),
        ExecuteMsg::WithdrawNft { token_id } => execute_withdraw_nft(deps, env, info, token_id),
        ExecuteMsg::GetReward {token_ids} =>execute_get_reward(deps, env, info, token_ids),
        ExecuteMsg::DistributeReward { token_balance } => execute_distribute_reward(deps,env,info,token_balance),
        ExecuteMsg::SetRewardWallet { address } => execute_reward_wallet(deps,env,info,address),
        ExecuteMsg::SetNftAddress { address } => execute_nft_address(deps,env,info,address),
        ExecuteMsg::SetTokenAddress { address } => execute_token_address(deps,env,info,address),
        ExecuteMsg::SetOwner { address } => execute_set_owner(deps, env, info, address),
        ExecuteMsg::WithdrawAllMoney { amount_juno,amount_hope } => execute_withdraw_all(deps,env, info, amount_juno,amount_hope),
        ExecuteMsg::SetStakingPeriod { time } => execute_staking_period(deps,env,info,time),
        ExecuteMsg::SetStake { flag } => execute_set_stake(deps,info,flag)
    }
}

fn execute_stake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    let token = TOKENINFO.may_load(deps.storage, &rcv_msg.token_id.clone())?;

    if state.can_stake == false{
        return Err(ContractError::CanNotStake{})
    }
    
    if info.sender.to_string() != state.nft_address{
        return Err(ContractError::WrongNftContract {  })
    }

    if token != None {
        return Err(ContractError::AlreadyStaked {  });
    }
   
    CONFIG.update(deps.storage,
        |mut state|->StdResult<_>{
            state.total_staked = state.total_staked + Uint128::new(1) ;
            Ok(state)
        }
    )?;

    let token_info = TokenInfo{
        owner:rcv_msg.sender,
        token_id:rcv_msg.token_id.clone(),
        status : "Staked".to_string(),
        unstake_time : 0,
        stake_time :env.block.time.seconds(),
        reward_juno: Uint128::new(0),
        reward_hope: Uint128::new(0)
    };

    TOKENINFO.save(deps.storage, &rcv_msg.token_id.clone(), &token_info)?;
    
    Ok(Response::default())

}



fn execute_unstake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    
) -> Result<Response, ContractError> {
    // let state = CONFIG.load(deps.storage)?;

    let token = TOKENINFO.may_load(deps.storage, &token_id)?;

    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();    

      if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

      TOKENINFO.update(deps.storage,&token_id,
        |token_info|->StdResult<_>{
            let mut token_info = token_info.unwrap();
            token_info.status = "Unstaking".to_string();
            token_info.unstake_time = env.block.time.seconds();
            
            Ok(token_info)
        }
    )?;
   }
    
    Ok(Response::default())

}

fn execute_withdraw_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    let token = TOKENINFO.may_load(deps.storage, &token_id)?;

    let mut messages:Vec<CosmosMsg> = vec![];
    
    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();

       if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

      if token.status =="Staked".to_string(){
          return Err(ContractError::StatusError {  })
      }

      if (env.block.time.seconds() - token.unstake_time)<state.staking_period{
           return Err(ContractError::TimeRemaining {  })
      
        }


      if token.reward_hope > Uint128::new(0){
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address, 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                  recipient: token.owner.clone(), 
                  amount: token.reward_hope 
                })? , 
             funds: vec![] }));
        }
       
      if token.reward_juno > Uint128::new(0){
      messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: token.owner,
                amount:vec![Coin{
                    denom:state.denom.clone(),
                    amount:token.reward_juno
                }]
        }));
    }
      TOKENINFO.remove(deps.storage,&token_id);
      CONFIG.update(deps.storage,
        |mut state|->StdResult<_>{
            state.total_staked = state.total_staked-Uint128::new(1);
            Ok(state)
        })?;
   }

   
    
  
   Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.nft_address, 
             msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                  recipient: info.sender.to_string(), 
                  token_id: token_id })? , 
             funds: vec![] }))
        .add_messages(messages)
         
)
}



fn execute_get_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_ids: Vec<String>,
    
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    let mut messages:Vec<CosmosMsg> = vec![];

    for token_id in token_ids{
    let token = TOKENINFO.may_load(deps.storage, &token_id)?;
    if token == None {
        return Err(ContractError::NotStaked {  });
    }
   
   else {
      let  token = token.unwrap();

       if token.owner != info.sender.to_string(){
          return Err(ContractError::Unauthorized {  })
      }

     if token.reward_hope > Uint128::new(0){
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address.clone(), 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                  recipient: token.owner.clone(), 
                  amount: token.reward_hope 
                })? , 
             funds: vec![] }));
        }
       
      if token.reward_juno > Uint128::new(0){
      messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: token.owner,
                amount:vec![Coin{
                    denom:state.denom.clone(),
                    amount:token.reward_juno
                }]
        }));
    }
      TOKENINFO.update(deps.storage,&token_id,
        |token_info|->StdResult<_>{
            let mut token_info = token_info.unwrap();
            token_info.reward_hope = Uint128::new(0);
            token_info.reward_juno = Uint128::new(0);
            Ok(token_info)
        })?;
   }
}
   
   Ok(Response::new()
        .add_messages(messages)
         
)
}

fn execute_distribute_reward(
    deps: DepsMut,
    env:  Env,
    info: MessageInfo,
    token_balance:Uint128
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.reward_wallet{
        return Err(ContractError::Unauthorized {});
    }

    if (env.block.time.seconds() - state.last_distribute)<state.staking_period{
        return Err(ContractError::CanNotDistribute {  })
    }

    let amount_juno= info
        .funds
        .iter()
        .find(|c| c.denom == state.denom)
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);
    
    let token_id :StdResult<Vec<String>>  = TOKENINFO
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();

    let token_group = token_id?;

   if token_group.len() == 0 {
       return Err(ContractError::NotStaked {  })
   }

   let mut reward_number = state.total_staked;

 
    for token_id in token_group.clone(){
        let token_info = TOKENINFO.load(deps.storage, &token_id )?;
        if  (env.block.time.seconds() - token_info.unstake_time) >state.staking_period && token_info.status == "Unstaking".to_string() {
          reward_number = reward_number - Uint128::new(1);   
        }
    }

    if reward_number == Uint128::new(0){
        return Err(ContractError::NotStaked {  })
    }
   

     for token_id in token_group{
            let token_info = TOKENINFO.load(deps.storage,&token_id)?;
            if token_info.status == "Staked".to_string() || (token_info.status == "Unstaking".to_string()&&(env.block.time.seconds() - token_info.unstake_time) <state.staking_period)
            {       TOKENINFO.update(deps.storage, &token_id,
                |token_info|->StdResult<_>{
                    let mut token_info = token_info.unwrap();
                    token_info.reward_hope = token_info.reward_hope + token_balance/reward_number;
                    token_info.reward_juno = token_info.reward_juno + amount_juno/reward_number;
                    Ok(token_info)
            }
            )?; }
    }

    CONFIG.update(deps.storage,
        |mut state|-> StdResult<_>{
            state.last_distribute = env.block.time.seconds();
            Ok(state)
        }    
    )?;

    Ok(Response::new()
    .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address, 
             msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                  owner:info.sender.to_string(),
                  recipient:env.contract.address.to_string(), 
                  amount: token_balance 
                })? , 
             funds: vec![] })))
}



fn execute_reward_wallet(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    address: String,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.reward_wallet = address;
        Ok(state)
    })?;
    Ok(Response::default())
}


fn execute_nft_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.nft_address = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}

fn execute_token_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.token_address = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}


fn execute_set_owner(
    deps: DepsMut,
     _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.owner = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}



fn execute_staking_period(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    time: u64,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.staking_period = time;
        Ok(state)
    })?;
    Ok(Response::default())
}


fn execute_set_stake(
    deps: DepsMut,

    info: MessageInfo,
    flag: bool,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage,
    |mut state|->StdResult<_>{
        state.can_stake = flag;
        Ok(state)
    })?;
    Ok(Response::default())
}



fn execute_withdraw_all(
    deps: DepsMut,
    _env : Env,
    info: MessageInfo,
    amount_juno: Uint128,
    amount_hope: Uint128
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
   
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: state.token_address, 
             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                  recipient: info.sender.to_string(), 
                  amount: amount_hope 
                })? , 
             funds: vec![] }))
        .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount:vec![Coin{
                    denom:state.denom.clone(),
                    amount:amount_juno
                }]
        }))
)
}






#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
          QueryMsg::GetStateInfo {  } => to_binary(&query_state_info(deps)?),
          QueryMsg::GetAllTokens {} => to_binary(&query_get_members(deps)?),
          QueryMsg::GetTokenInfo {} => to_binary(&query_token_info(deps)?),
          QueryMsg::GetCurrentTime{} => to_binary(&query_get_current_time(deps,_env)?),
    }
}

pub fn query_state_info(deps:Deps) -> StdResult<State>{
    let state =  CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_get_current_time(deps:Deps,env:Env) -> StdResult<u64>{
    Ok(env.block.time.seconds())
}

pub fn query_get_members(deps:Deps) -> StdResult<Vec<String>>{
     let token_id :StdResult<Vec<String>>  = TOKENINFO
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(token_id?)
}

pub fn query_token_info(deps:Deps)->StdResult<Vec<TokenInfo>>{
      let res: StdResult<Vec<TokenInfo>> = TOKENINFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item|parse_token_info(kv_item))
        .collect();
    Ok(res?)
}

fn parse_token_info(
    item: StdResult<(String,TokenInfo)>,
) -> StdResult<TokenInfo> {
    item.and_then(|(k, token_info)| {
        Ok(token_info)
    })
}




#[cfg(test)]
mod tests {

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{CosmosMsg};

    #[test]
    fn testing() {
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
            denom : "ujuno".to_string(),
            staking_period : 1000,
            reward_wallet :"reward_wallet".to_string()
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());


        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state,State{
            nft_address : "nft_address".to_string(),
            token_address : "token_address".to_string(),
            owner:"creator".to_string(),
            staking_period : 1000,
            denom : "ujuno".to_string(),
            reward_wallet:"reward_wallet".to_string(),
            total_staked:Uint128::new(0),
            can_stake : true,
            last_distribute : mock_env().block.time.seconds()
        });

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetNftAddress { address:"nft_address1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetTokenAddress { address:"token_address1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.nft_address,"nft_address1".to_string());
        assert_eq!(state.token_address, "token_address1".to_string());

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetRewardWallet { address:"reward_wallet1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.reward_wallet,"reward_wallet1".to_string());
       

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::SetStakingPeriod { time: 1200 };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.staking_period,1200);

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id : "reveal1".to_string(),
            msg : to_binary(&"abc".to_string()).unwrap()
        });
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id : "reveal2".to_string(),
            msg : to_binary(&"abc".to_string()).unwrap()
        });
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal1","reveal2"]);

        let info = mock_info("owner1", &[]);
        let msg = ExecuteMsg::UnstakeNft { token_id : "reveal1".to_string() };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal1","reveal2"]);

        let token_infos = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Unstaking".to_string(),
            reward_hope:Uint128::new(0),
            reward_juno:Uint128::new(0),
            unstake_time : mock_env().block.time.seconds()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward_hope:Uint128::new(0),
            reward_juno:Uint128::new(0),
            unstake_time :0
        }]);

        let info = mock_info("reward_wallet1", &[]);     
        let msg = ExecuteMsg::DistributeReward { token_balance:Uint128::new(0)  };
        execute(deps.as_mut(),mock_env(),info,msg).unwrap();

        let token_infos = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(token_infos,vec![TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal1".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Unstaking".to_string(),
            reward_hope:Uint128::new(0),
            reward_juno:Uint128::new(0),
            unstake_time : mock_env().block.time.seconds()
        },TokenInfo{
            owner:"owner1".to_string(),
            token_id:"reveal2".to_string(),
            stake_time:mock_env().block.time.seconds(),
            status:"Staked".to_string(),
            reward_hope:Uint128::new(0),
            reward_juno:Uint128::new(0),
            unstake_time :0
        }]);

        let info = mock_info("owner1", &[]);     
        let msg = ExecuteMsg::GetReward { token_ids:vec!["reveal1".to_string(),"reveal2".to_string()] };
        let res = execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        assert_eq!(0,res.messages.len());
        // assert_eq!(res.messages[0].msg,CosmosMsg::Bank(BankMsg::Send {
        //         to_address: "owner1".to_string(),
        //         amount:vec![Coin{
        //             denom:state.denom.clone(),
        //             amount:Uint128::new(5)
        //         }]
        // }));

       
        
        let info = mock_info("owner1", &[]);     
        let msg = ExecuteMsg::WithdrawNft { token_id:"reveal1".to_string() };
        let res = execute(deps.as_mut(),mock_env(),info,msg).unwrap();
        assert_eq!(1,res.messages.len());
        assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
             contract_addr: "nft_address1".to_string(), 
             msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                  recipient: "owner1".to_string(), 
                  token_id: "reveal1".to_string() }).unwrap() , 
             funds: vec![] }));

        let tokens = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(tokens,vec!["reveal2"]);

    }
}
