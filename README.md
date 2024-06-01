[![Rust](https://github.com/brytelands/sologger-geyser-plugin/actions/workflows/rust.yml/badge.svg)](https://github.com/brytelands/sologger-geyser-plugin/actions/workflows/rust.yml)

# sologger-geyser-plugin

Configurable Solana Geyser plugin that uses [Sologger](https://github.com/brytelands/sologger) to structure Solana validator logs and send them to either Logstash or an OpenTelemetry collector.
Sologger parses raw logs ingested by the geyser plugin into structured logs and transports Solana logs to either a LogStash or OpenTelemetry endpoint via TCP. This helps improve the observability of your programs running on chain.

### Quick Start

If you just want to run a Solana test validator with the Sologger Geyser plugin, then you can use one of the following docker compose files to get up and running quickly.

- [sologger with Parseable](./docker-examples/docker-parseable/) This is the easiest way to get up and running with Sologger. If you want to monitor specific programs, all you need to do is update the program IDs in the sologger-config.json file.
- [sologger with Signoz](./docker-examples/docker-signoz/) This is an example using OpenTelemetry. It's a bit more involved and takes a little bit of time to startup Signoz. If you want to monitor specific programs, all you need to do is update the program IDs in the sologger-config.json file.


**Building the source**
There are two main features that can be enabled when building the plugin binaries. The first is the Logstash feature which will enable the Logstash transport. The second is the OpenTelemetry feature which will enable the OpenTelemetry transport. Technically you can build with both features enabled, but this is not recommended. If you need both LogStash and OTel support, the recommended approach is to build two binaries with each feature enabled, and run each separately.

- Logstash: `enable_logstash`
- OpenTelemetry: `enable_otel`

```shell
#If you want to build the binaries with Logstash support, then run the following command:
cargo build --features 'enable_logstash'

#If you want to build the binaries with OpenTelemetry support, then run the following command:
cargo build --features 'enable_otel'
```

**Building the docker image**

These images are basically just the Solana CLI that starts a test-validator with the Sologger Geyser Plugin installed.

```shell
#If you want to build the image with Logstash support, then run the following command:
docker build -f 'Dockerfile-logstash' --tag sologger-logstash-geyser-plugin .

#If you want to build the image with OpenTelemetry support, then run the following command:
docker build --file 'Dockerfile-otel' --tag sologger-otel-geyser-plugin .
```

### Configure

There are two configuration files that you will need to configure to get up and running with Sologger.
The first is the sologger-config file. This file is used to configure the sologger binary.
The second is the log4rs-config file. This file is used to configure the log4rs logger OR the opentelemetry-config file. This file is used to configure the logstash binary.

By default, sologger will look for a config file named `sologger-config.json` in ./config/local/ directory. You can override this by setting the `SOLOGGER_APP_CONFIG_LOCATION` environment variable to the path of your config file. For example:

Here is an example sologger-config.json. See [sologger_config.rs](src/sologger_config.rs) for documentation specific to each field.
```json
{
    "log4rsConfigLocation": "../config/local/log4rs-config.yml",
    "opentelemetryConfigLocation": "../config/local/opentelemetry-config.json",
    "rpcUrl": "wss://api.devnet.solana.com",
    "programsSelector" : {
        "programs" : ["BPFLoaderUpgradeab1e11111111111111111111111", "Ed25519SigVerify111111111111111111111111111", "KeccakSecp256k11111111111111111111111111111"]
    },
    "accountDataNotificationsEnabled": false,
    "transactionNotificationsEnabled": true,
    "logProcessorWorkerThreadCount": 2
}
```

For log4rs configurations please see: [log4rs](https://github.com/estk/log4rs)


### Run

You can also take a look at the run scripts in [scripts](./scripts)

```shell
solana-test-validator --geyser-plugin-config ../config/sologger-geyser-plugin-config.json

#Or if you want to specify a location of the sologger-config.json
SOLOGGER_APP_CONFIG_LOCATION=./config/sologger-config.json solana-test-validator --geyser-plugin-config ../config/sologger-geyser-plugin-config.json
```

Or

```shell
#If you just want to run the docker image
sudo docker run --name solana-test-validator-sologger-otel -p 1024:1024 -p 9900:9900 -p 8900:8900 -p 8899:8899 sologger-otel-geyser-plugin
```

Or

Start up the Parseable stack using docker-compose

```shell
docker-compose -f docker-examples/docker-parseable/docker-compose.yml up
```

You can deploy your program and see the logs here

- url: http://localhost:8000
- user: admin
- password: admin