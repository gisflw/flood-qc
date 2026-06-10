FROM python:3.12-bookworm

RUN apt-get update && apt-get install -y \
    git curl ca-certificates build-essential \
    nodejs npm \
    gdal-bin libgdal-dev sqlite3 \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g @openai/codex

RUN useradd -m -s /bin/bash vscode
USER vscode

WORKDIR /workspace