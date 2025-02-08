# `bolt`

This project is a Rust implementation of a Nostr relay

## Project Spec & Dependencies

- Language
  - Rust `1.84.1`
- Tools and Frameworks
  - axum `0.8.1` for HTTP and WebSocket server, routing, and middleware
  - tokio `1.43.0` for async runtime
  - futures `0.3.31` for async streams
  - tower-http `0.6.2` for HTTP server and middleware
  - tower `0.5.2` for middleware
  - serde `1.0.217` for serializing and deserializing JSON
  - serde_json `1.0.138` for parsing JSON
  - serde_yaml `0.0.6` for parsing YAML
  - nostr `0.39.0` for Nostr protocol implementation

## Project Structure

- `src/main.rs` - Entry point for the application
- `src/lib.rs` - Library code for the application
- `src/config.rs` - Configuration for the application

## Configuration

- `config.yml` - Configuration for the application

## Nostr Protocol Documentation

- [Nostr NIPs](https://github.com/nostr-protocol/nips)
