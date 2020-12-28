FROM rust:latest

RUN mkdir /code
WORKDIR /code
COPY ./ /code/