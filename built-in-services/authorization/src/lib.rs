mod types;

use binding_macro::{cycles, genesis, service};
use protocol::traits::{ExecutorParams, ServiceResponse, ServiceSDK, StoreMap};
use protocol::types::{Address, ServiceContext, SignedTransaction};

use crate::types::{AddVerifiedItemPayload, InitGenesisPayload, RemoveVerifiedItemPayload};

const AUTHORIZATION_ADMIN_KEY: &str = "authotization_admin";

pub struct AuthorizationService<SDK> {
    sdk:          SDK,
    verified_map: Box<dyn StoreMap<String, String>>,
}

#[service]
impl<SDK: ServiceSDK> AuthorizationService<SDK> {
    pub fn new(mut sdk: SDK) -> Self {
        let verified_map: Box<dyn StoreMap<String, String>> =
            sdk.alloc_or_recover_map("authotization");

        Self { sdk, verified_map }
    }

    #[genesis]
    fn init_genesis(&mut self, payload: InitGenesisPayload) {
        let service_names = payload.register_service_names;
        let function_names = payload.verified_method_names;
        assert!(service_names.len() == function_names.len());

        self.sdk
            .set_value(AUTHORIZATION_ADMIN_KEY.to_string(), payload.admin);

        for item in service_names.into_iter().zip(function_names.into_iter()) {
            self.verified_map.insert(item.0, item.1);
        }
    }

    #[cycles(210_00)]
    #[read]
    fn check_authorization(
        &self,
        ctx: ServiceContext,
        payload: SignedTransaction,
    ) -> ServiceResponse<()> {
        let payload_json = match serde_json::to_string(&payload) {
            Ok(json_string) => json_string,
            Err(_) => {
                return ServiceResponse::<()>::from_error(
                    101,
                    "serialize transaction to json error".to_owned(),
                )
            }
        };

        for (service_name, mathod_name) in self.verified_map.iter() {
            if self
                ._do_verify(&ctx, &service_name, &mathod_name, &payload_json)
                .is_error()
            {
                return ServiceResponse::<()>::from_error(
                    102,
                    format!("verify transaction {:?} error", mathod_name),
                );
            }
        }

        ServiceResponse::from_succeed(())
    }

    #[cycles(210_00)]
    #[write]
    fn add_verified_item(
        &mut self,
        ctx: ServiceContext,
        payload: AddVerifiedItemPayload,
    ) -> ServiceResponse<()> {
        if !self.is_admin(&ctx) {
            return ServiceResponse::<()>::from_error(103, "Invalid caller".to_owned());
        }

        self.verified_map
            .insert(payload.service_name, payload.method_name);
        ServiceResponse::from_succeed(())
    }

    #[cycles(210_00)]
    #[write]
    fn remove_verified_item(
        &mut self,
        ctx: ServiceContext,
        payload: RemoveVerifiedItemPayload,
    ) -> ServiceResponse<()> {
        if !self.is_admin(&ctx) {
            return ServiceResponse::<()>::from_error(103, "Invalid caller".to_owned());
        }

        self.verified_map.remove(&payload.service_name);
        ServiceResponse::from_succeed(())
    }

    fn _do_verify(
        &self,
        ctx: &ServiceContext,
        service_name: &str,
        method_name: &str,
        payload_json: &str,
    ) -> ServiceResponse<String> {
        self.sdk
            .read(&ctx, None, service_name, method_name, &payload_json)
    }

    fn is_admin(&self, ctx: &ServiceContext) -> bool {
        let admin: Address = self
            .sdk
            .get_value(&AUTHORIZATION_ADMIN_KEY.to_string())
            .expect("must have an admin");
        ctx.get_caller() == admin
    }
}
