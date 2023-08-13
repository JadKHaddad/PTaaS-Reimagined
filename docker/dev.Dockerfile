FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    curl \
    python3.11 \
    python3.11-venv \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

WORKDIR /app

CMD ["/bin/bash"]

# docker build -t ptaas_dev -f docker/dev.Dockerfile .
# docker run -it --rm -v ${pwd}:/app ptaas_dev