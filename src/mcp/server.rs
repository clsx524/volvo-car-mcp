use std::path::PathBuf;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use serde::Serialize;

use crate::commands::{
    auth_whoami, energy_get, location_get, vehicle_commands, vehicle_get, vehicle_shared,
    vehicle_vin,
};
use crate::config::DEFAULT_API_HOST;
use crate::store::sqlite::{Profile, Store};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VinArgs {
    /// Optional VIN. Omit to use the stored default VIN for the active profile.
    pub vin: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EngineStartArgs {
    /// Optional VIN. Omit to use the stored default VIN for the active profile.
    pub vin: Option<String>,
    /// Engine runtime in minutes.
    pub runtime_minutes: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SetDefaultVinArgs {
    /// VIN to store as the profile default. Must be discoverable via the vehicle list.
    pub vin: String,
}

#[derive(Clone)]
pub struct VolvoMcp {
    store_path: PathBuf,
    profile_name: String,
    base_url: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl VolvoMcp {
    pub fn new(store_path: PathBuf, profile_name: String, base_url: String) -> Self {
        Self {
            store_path,
            profile_name,
            base_url,
            tool_router: Self::tool_router(),
        }
    }

    fn open_store(&self) -> Result<(Store, Profile), McpError> {
        let store = Store::open(&self.store_path).map_err(|err| {
            McpError::internal_error(format!("failed to open state store: {err:#}"), None)
        })?;
        let profile = store
            .get_or_create_profile(&self.profile_name, DEFAULT_API_HOST)
            .map_err(|err| {
                McpError::internal_error(format!("failed to resolve profile: {err:#}"), None)
            })?;
        Ok((store, profile))
    }

    fn finish<T: Serialize>(
        &self,
        result: Result<T, anyhow::Error>,
    ) -> Result<CallToolResult, McpError> {
        match result {
            Ok(output) => {
                let text = serde_json::to_string_pretty(&output).map_err(|err| {
                    McpError::internal_error(
                        format!("failed to serialize tool output: {err}"),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
            }
            Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "{err:#}"
            ))])),
        }
    }

    #[tool(
        description = "List all vehicles linked to the authenticated Volvo ID account. Returns the VINs available to this profile; call this first to discover vehicles."
    )]
    async fn vehicle_list(&self) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_list(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleApiArgs { api_key: None },
            )
            .await,
        )
    }

    #[tool(description = "Get generic vehicle properties (model, model year, colour) for a VIN.")]
    async fn vehicle_details(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Details,
            )
            .await,
        )
    }

    #[tool(description = "Get the open/closed status of each vehicle window.")]
    async fn vehicle_windows(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Windows,
            )
            .await,
        )
    }

    #[tool(description = "Get the doors and central lock status of the vehicle.")]
    async fn vehicle_doors(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Doors,
            )
            .await,
        )
    }

    #[tool(description = "Get vehicle warning indicator statuses.")]
    async fn vehicle_warnings(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Warnings,
            )
            .await,
        )
    }

    #[tool(description = "Get tyre pressure statuses for the vehicle.")]
    async fn vehicle_tyres(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Tyres,
            )
            .await,
        )
    }

    #[tool(description = "Get trip and usage statistics for the vehicle.")]
    async fn vehicle_statistics(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Statistics,
            )
            .await,
        )
    }

    #[tool(description = "Get the odometer reading of the vehicle.")]
    async fn vehicle_odometer(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Odometer,
            )
            .await,
        )
    }

    #[tool(description = "Get the latest fuel amount and/or battery charge level of the vehicle.")]
    async fn vehicle_fuel(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Fuel,
            )
            .await,
        )
    }

    #[tool(description = "Get the vehicle diagnostics overview (fluid levels, service status).")]
    async fn vehicle_diagnostics(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Diagnostics,
            )
            .await,
        )
    }

    #[tool(description = "Get engine diagnostic values for the vehicle.")]
    async fn vehicle_engine_diagnostics(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::EngineDiagnostics,
            )
            .await,
        )
    }

    #[tool(description = "Get the latest engine status (running or not) for the vehicle.")]
    async fn vehicle_engine_status(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::EngineStatus,
            )
            .await,
        )
    }

    #[tool(description = "Get the brake status (brake fluid level) of the vehicle.")]
    async fn vehicle_brakes(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_get::execute_vehicle_read(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_get::VehicleReadEndpoint::Brakes,
            )
            .await,
        )
    }

    #[tool(
        description = "Get the latest energy state (battery charge level, charging status, electric range) for a BEV/PHEV vehicle."
    )]
    async fn energy_state(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            energy_get::execute(
                &store,
                &profile,
                &self.base_url,
                energy_get::EnergyGetArgs { vin, api_key: None },
                energy_get::EnergyReadEndpoint::State,
            )
            .await,
        )
    }

    #[tool(
        description = "Get per-vehicle energy API capability metadata (which energy fields the vehicle supports)."
    )]
    async fn energy_capabilities(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            energy_get::execute(
                &store,
                &profile,
                &self.base_url,
                energy_get::EnergyGetArgs { vin, api_key: None },
                energy_get::EnergyReadEndpoint::Capabilities,
            )
            .await,
        )
    }

    #[tool(description = "Get the latest known location of the vehicle.")]
    async fn location_get(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            location_get::execute(
                &store,
                &profile,
                &self.base_url,
                location_get::LocationGetArgs { vin, api_key: None },
            )
            .await,
        )
    }

    #[tool(description = "List the remote commands the vehicle supports (read-only metadata).")]
    async fn commands_list(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_query(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleCommandsQueryEndpoint::List,
            )
            .await,
        )
    }

    #[tool(
        description = "Check whether remote commands are currently accessible for the vehicle (read-only; e.g. some commands need a locked or parked car)."
    )]
    async fn commands_accessibility(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_query(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleCommandsQueryEndpoint::Accessibility,
            )
            .await,
        )
    }

    #[tool(description = "PHYSICAL ACTUATION: lock the vehicle. Requires explicit user intent.")]
    async fn lock(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::Lock,
            )
            .await,
        )
    }

    #[tool(description = "PHYSICAL ACTUATION: unlock the vehicle. Requires explicit user intent.")]
    async fn unlock(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::Unlock,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: lock the vehicle with reduced guard (interior motion sensors stay disarmed). Requires explicit user intent."
    )]
    async fn lock_reduced_guard(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::LockReducedGuard,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: sound the vehicle horn. Requires explicit user intent."
    )]
    async fn honk(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::Honk,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: flash the vehicle exterior lights. Requires explicit user intent."
    )]
    async fn flash(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::Flash,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: sound the horn and flash the exterior lights at the same time. Requires explicit user intent."
    )]
    async fn honk_flash(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::HonkFlash,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: remotely start the engine for the given number of minutes. Requires explicit user intent."
    )]
    async fn engine_start(
        &self,
        Parameters(EngineStartArgs {
            vin,
            runtime_minutes,
        }): Parameters<EngineStartArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_engine_start(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                runtime_minutes,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: remotely stop the engine. Requires explicit user intent."
    )]
    async fn engine_stop(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::EngineStop,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: start vehicle climatization (remote pre-heating or cooling). Requires explicit user intent."
    )]
    async fn climatization_start(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::ClimatizationStart,
            )
            .await,
        )
    }

    #[tool(
        description = "PHYSICAL ACTUATION: stop vehicle climatization. Requires explicit user intent."
    )]
    async fn climatization_stop(
        &self,
        Parameters(VinArgs { vin }): Parameters<VinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(
            vehicle_commands::execute_invoke(
                &store,
                &profile,
                &self.base_url,
                vehicle_shared::VehicleVinApiArgs { vin, api_key: None },
                vehicle_commands::VehicleInvokeEndpoint::ClimatizationStop,
            )
            .await,
        )
    }

    #[tool(
        description = "Show profile identity, session expiry, requested scopes, cached VINs and the default VIN. No network call."
    )]
    async fn auth_whoami(&self) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(auth_whoami::execute(&store, &profile, &self.base_url))
    }

    #[tool(
        description = "Store a VIN as the profile default. Validates the VIN against the vehicle list endpoint; run once if the account has multiple vehicles."
    )]
    async fn set_default_vin(
        &self,
        Parameters(SetDefaultVinArgs { vin }): Parameters<SetDefaultVinArgs>,
    ) -> Result<CallToolResult, McpError> {
        let (store, profile) = self.open_store()?;
        self.finish(vehicle_vin::set_default(&store, &profile, &self.base_url, None, &vin).await)
    }
}

#[tool_handler(router = self.tool_router.clone())]
impl ServerHandler for VolvoMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "volvo-car-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_protocol_version(ProtocolVersion::V_2025_06_18)
            .with_instructions(
                "Volvo Cars Connected Vehicle, Energy and Location API tools. \
                 VIN-optional tools fall back to the stored default VIN: call vehicle_list \
                 first, then set_default_vin once if several vehicles exist. \
                 lock, unlock, lock_reduced_guard, honk, flash, honk_flash, engine_start, \
                 engine_stop, climatization_start and climatization_stop physically actuate \
                 the vehicle and require explicit user intent. Volvo rate limits: 10 \
                 command calls per minute, 100 reads per minute, 10000 calls per day.",
            )
    }
}
