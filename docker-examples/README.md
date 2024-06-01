## Log Management examples with Docker

**docker-parseable**

This is the most straightforward way to play around with Sologger or quickly get up and running for development or testing purposes.
This is a simple example of how to use the sologger-logstash image to send logs to Logstash, which then sends the logs to Parseable.

**docker-signoz**

This is a example of how to use Signoz to manage Solana logs. It uses the sologger-otel to an OpenTelemetry collector provided by Signoz.