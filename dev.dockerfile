FROM ubuntu:22.04

WORKDIR /root/
COPY . /root/

RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
      libelf1 libelf-dev zlib1g-dev libclang-13-dev \
      make git clang llvm pkg-config build-essential && \
    apt-get install -y --no-install-recommends ca-certificates	&& \
	  update-ca-certificates	&& \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/bin/bash"]
