#![allow(dead_code)]
pub mod game;
pub mod resource;
pub mod sprite;
pub mod scene;
pub mod info;
pub mod effect;

#[cfg(feature="firestorm-cpu")]
pub(crate) use firestorm::{
     profile_fn,
     profile_method,
     profile_section
};

#[cfg(not(feature="firestorm-cpu"))]
macro_rules! profile_fn{    
    ($a:expr)=>{};
}
#[cfg(not(feature="firestorm-cpu"))]
macro_rules! profile_section{    
    ($a:ident)=>{
        #[allow(unused_variables)]
        let $a=0;
    };
}
#[cfg(not(feature="firestorm-cpu"))]
macro_rules! profile_method{    
    ($a:expr)=>{};
}
#[cfg(not(feature="firestorm-cpu"))]
pub(crate) use {
    profile_fn,
    profile_method,
    profile_section    
};