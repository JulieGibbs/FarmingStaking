use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Wrong nft contract error")]
    WrongNftContract {},

    
    #[error("Not enough funds")]
    Notenough{},

     #[error("Alreay staked")]
    AlreadyStaked{},

     #[error("Not staked")]
    NotStaked{},
 
    #[error("Time remaining yet")]
    TimeRemaining {},

    
    #[error("Can not stake")]
    CanNotStake {},

    #[error("Can not distribute")]
    CanNotDistribute {},

    
    #[error("Stkaing process")]
    StatusError {},
}
