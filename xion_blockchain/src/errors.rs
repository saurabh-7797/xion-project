use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Custom Error: {message}")]
    CustomError { message: String },

    #[error("Username is invalid")]
    InvalidUsername {},

    #[error("Username is already taken")]
    UsernameTaken {},

    #[error("Not token owner")]
    NotTokenOwner {},
} 