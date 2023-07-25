# PTaaS - Performance Testing as a Service
Test your SUT's (System Under Test) performance using Locust scripts.

## Description
This project is a reimplementation of [Rust-Performance-Testing-as-a-Service](https://github.com/JadKHaddad/Rust-Performance-Testing-as-a-Service) and [Openfaas-Performance-Testing-as-a-Service](https://github.com/JadKHaddad/Openfaas-Performance-Testing-as-a-Service) focused on extensibility, scalability, modularity and flexibility. The backend is written completely in Rust to ensure performance and safety. The frontend is written with flutter to ensure cross platform compatibility and flexibility.

## System
- [ ] Backend
    - [ ] Prometheus metrics: for trafic and performance
    - [ ] Databases
        - [ ] Json
        - [ ] Sqlite
        - [ ] Postgres
    - [ ] Project managers: with Prometheus metrics for projects and tests statistics
        - [ ] Local project manager: standalone
        - [ ] Distributed Local project manager: with local dispatcher
        - [ ] Docker project manager: standalone
            - [ ] Dockerfiles
                - [ ] Base image: python3.11 and python3.11-venv
                - [ ] Base project image: a project image with all dependencies
                - [ ] Script image: a project image with a script runner cli
        - [ ] K8s project manager: with k8s dispatcher
    - [ ] Dispatchers
        - [ ] Local dispatcher
        - [ ] K8s dispatcher
    - [ ] Connection manager
    - [ ] Script runner
- [ ] Script runner cli: standalone: with a server to export results
- [ ] API models converter
- [ ] Frontend
    - [ ] API
    - [ ] Websocket with polling fallback
- [ ] Helmchart