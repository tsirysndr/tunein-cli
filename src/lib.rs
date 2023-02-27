pub mod api {
    #[path = ""]
    pub mod tunein {
        #[path = "tunein.v1alpha1.rs"]
        pub mod v1alpha1;
    }

    #[path = ""]
    pub mod objects {
        #[path = "objects.v1alpha1.rs"]
        pub mod v1alpha1;
    }
}
