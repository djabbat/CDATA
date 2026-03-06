use thiserror::Error;
use hecs::ComponentError;

#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("Module error: {0}")]
    ModuleError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Entity not found")]
    EntityNotFound,
    
    #[error("Component not found")]
    ComponentNotFound,
    
    #[error("Component error: {0}")]
    ComponentError(#[from] ComponentError),
    
    #[error("No such entity")]
    NoSuchEntity,
}

impl From<hecs::NoSuchEntity> for SimulationError {
    fn from(_: hecs::NoSuchEntity) -> Self {
        SimulationError::NoSuchEntity
    }
}

pub type SimulationResult<T> = Result<T, SimulationError>;
