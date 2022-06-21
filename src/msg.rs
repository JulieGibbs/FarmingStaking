use cosmwasm_std::{ Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw721::Cw721ReceiveMsg;



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub denom:String,
    pub staking_period : u64,
    pub reward_wallet : String,
    pub distribute_period: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    UnstakeNft{token_id:String},
    WithdrawNft{token_id:String},
    GetReward{token_ids:Vec<String>},  
    DistributeReward{},
    SetRewardWallet{address:String},
    SetOwner{address:String},
    SetStakingPeriod{time:u64},   
    WithdrawAllMoney{amount_juno:Uint128},
    SetNftAddress{address:String},
    SetTokenAddress{address:String},
    SetStake{flag:bool},
    SetDistributePeriod{time:u64}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  GetStateInfo{},
  GetAllTokens{},
  GetTokenInfo{},
  GetCurrentTime{},
  GetToken{token_id:String},
  GetMyIds{address:String},
  GetMyInfo{address:String}
}

