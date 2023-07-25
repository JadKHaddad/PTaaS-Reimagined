# PTaaS - Performance Testing as a Service

## Description


## System
- [ ] Backend
    - [ ] Databases
        - [ ] Json
        - [ ] Sqlite
        - [ ] Postgres
    - [ ] Project managers
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
- [ ] Script runner cli: standalone
- [ ] API models converter
- [ ] Frontend
    - [ ] API
    - [ ] Websocket with polling fallback
- [ ] Helmchart