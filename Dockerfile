FROM ubuntu:24.04
ARG TARGETARCH

WORKDIR /app

# Noninteractive to avoid tzdata prompts
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    sudo \
    smartmontools \
    nvme-cli \
    curl \
    iproute2 \
    procps \
    && rm -rf /var/lib/apt/lists/*

COPY web/dist ./web/dist
COPY build/linux-${TARGETARCH}/wadm ./wadm
RUN chmod +x ./wadm

EXPOSE 8080

CMD ["./wadm"]
