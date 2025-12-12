pub mod sui {
    pub mod rpc {
        pub mod v2 {
            include!(concat!(env!("OUT_DIR"), "/sui.rpc.v2.rs"));
        }
    }
}
