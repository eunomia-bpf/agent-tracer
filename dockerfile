FROM ubuntu:22.04

WORKDIR /root/
COPY . /root/

RUN apt-get update \
    && apt-get install -y --no-install-recommends libelf1 \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/root/src/bootstrap"]
