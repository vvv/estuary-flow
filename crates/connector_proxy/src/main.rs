use clap::Parser;
use flow_cli_common::{init_logging, LogArgs};

pub mod apis;
pub mod connector_runner;
pub mod errors;
pub mod interceptors;
pub mod libs;

use apis::{FlowCaptureOperation, FlowMaterializeOperation, FlowRuntimeProtocol};
use connector_runner::{
    run_airbyte_source_connector, run_flow_capture_connector, run_flow_materialize_connector,
};
use errors::Error;
use libs::image_inspect::ImageInspect;

#[derive(Debug, clap::ArgEnum, Clone)]
pub enum CaptureConnectorProtocol {
    Airbyte,
    FlowCapture,
}

#[derive(Debug, clap::Parser)]
struct ProxyFlowCapture {
    /// The operation (in the FlowCapture Protocol) that is being proxied.
    #[clap(arg_enum)]
    operation: FlowCaptureOperation,
}

#[derive(Debug, clap::ArgEnum, Clone)]
pub enum MaterializeConnectorProtocol {
    FlowMaterialize,
}

#[derive(Debug, clap::Parser)]
struct ProxyFlowMaterialize {
    /// The operation (in the FlowMaterialize Protocol) that is being proxied.
    #[clap(arg_enum)]
    operation: FlowMaterializeOperation,
}

#[derive(Debug, clap::Subcommand)]
enum ProxyCommand {
    /// proxies the Flow runtime Capture Protocol to the connector.
    ProxyFlowCapture(ProxyFlowCapture),
    /// proxies the Flow runtime Materialize Protocol to the connector.
    ProxyFlowMaterialize(ProxyFlowMaterialize),
}

#[derive(clap::Parser, Debug)]
#[clap(about = "Command to start connector proxies for Flow runtime.")]
pub struct Args {
    /// The path (in the container) to the JSON file that contains the inspection results from the connector image.
    /// Normally produced via command "docker inspect <image>".
    #[clap(short, long)]
    image_inspect_json_path: Option<String>,

    /// The type of proxy service to provide.
    #[clap(subcommand)]
    proxy_command: ProxyCommand,

    #[clap(flatten)]
    log_args: LogArgs,
}

static DEFAULT_CONNECTOR_ENTRYPOINT: &str = "/connector/connector";

// The connector proxy is a service between Flow Runtime and connectors. It adapts the protocol of the Flow Runtime to
// to protocol of the connector, and allows additional functionalities to be triggered during the communications.
// 1. The protocol of Flow Runtime is determined by proxyCommand,
//    "proxy-flow-capture" and "proxy-flow-materialize" for FlowCapture and FlowMaterialize protocols, respectively.
// 2. The interceptors translate the Flow Runtime protocols to the native protocols of different connectors, and add functionalities
//    that affect multiple operations during the communication. E.g. network proxy needs to modify the spec response to add the
//    network proxy specs, and starts the network proxy for the rest commands. See apis.rs for details.nd allows additional
//    functionalities to be triggered during the communications.

#[tokio::main]
async fn main() {
    let Args {
        image_inspect_json_path,
        proxy_command,
        log_args,
    } = Args::parse();
    init_logging(&log_args);

    let result = async_main(image_inspect_json_path, proxy_command).await;

    match result {
        Err(Error::CommandExecutionError(_)) => {
            // This error summarizes an error of a child process.
            // As its stderr is passed through, we don't log its failure again here.
            std::process::exit(1);
        }
        Err(err) => {
            tracing::error!(error = ?err, message = "connector-proxy failed");
            std::process::exit(1);
        }
        Ok(()) => {
            tracing::info!(message = "connector-proxy exiting");
        }
    }
}

async fn async_main(
    image_inspect_json_path: Option<String>,
    proxy_command: ProxyCommand,
) -> Result<(), Error> {
    match proxy_command {
        ProxyCommand::ProxyFlowCapture(c) => proxy_flow_capture(c, image_inspect_json_path).await,
        ProxyCommand::ProxyFlowMaterialize(m) => {
            proxy_flow_materialize(m, image_inspect_json_path).await
        }
    }
}

async fn proxy_flow_capture(
    c: ProxyFlowCapture,
    image_inspect_json_path: Option<String>,
) -> Result<(), Error> {
    let image_inspect = ImageInspect::parse_from_json_file(image_inspect_json_path)?;
    if image_inspect.infer_runtime_protocol() != FlowRuntimeProtocol::Capture {
        return Err(Error::MismatchingRuntimeProtocol);
    }

    let entrypoint = image_inspect.get_entrypoint(vec![DEFAULT_CONNECTOR_ENTRYPOINT.to_string()]);

    match image_inspect
        .get_connector_protocol::<CaptureConnectorProtocol>(CaptureConnectorProtocol::Airbyte)
    {
        CaptureConnectorProtocol::FlowCapture => {
            run_flow_capture_connector(&c.operation, entrypoint).await
        }
        CaptureConnectorProtocol::Airbyte => {
            run_airbyte_source_connector(&c.operation, entrypoint).await
        }
    }
}

async fn proxy_flow_materialize(
    m: ProxyFlowMaterialize,
    image_inspect_json_path: Option<String>,
) -> Result<(), Error> {
    let image_inspect = ImageInspect::parse_from_json_file(image_inspect_json_path)?;
    if image_inspect.infer_runtime_protocol() != FlowRuntimeProtocol::Materialize {
        return Err(Error::MismatchingRuntimeProtocol);
    }

    let entrypoint = image_inspect.get_entrypoint(vec![DEFAULT_CONNECTOR_ENTRYPOINT.to_string()]);

    run_flow_materialize_connector(&m.operation, entrypoint).await
}
