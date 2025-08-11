# REST API with [Axum](https://docs.rs/axum/latest/axum/)

## Features
* Create/Read/Update/Aggregate/Stream a resource(s) (User).
* Use Trait object to abstract [Database layer](https://github.com/darrell-roberts/rust-web-proto/tree/master/user-database).
* Use [MongoDB](https://docs.rs/mongodb/latest/mongodb/) database implementation for runtime and mock database implementation for unit testing.
* JSON data extractor that validates deserialized types using the [validator crate](https://docs.rs/validator/latest/validator/index.html).
* SSL Server.
* SSL mutual TLS with [MongoDB](https://docs.rs/mongodb/latest/mongodb/).
* JWT authorization with [jsonwebtoken](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/)
* Thread a x-request-id header through each request.
* Request logging with [tracing](https://docs.rs/tracing/latest/tracing/)
* Middleware with [tower-http](https://docs.rs/tower-http/latest/tower_http/)
* Middleware layer apply user defined hashing to responses
