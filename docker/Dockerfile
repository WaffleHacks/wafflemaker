FROM python:3-alpine

RUN pip3 install --no-cache-dir requests

COPY ./scripts ./

ENTRYPOINT [ "python3", "./vault.py" ]
