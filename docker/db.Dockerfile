FROM postgres:16

ENV POSTGRES_USER=horariosUser
ENV POSTGRES_PASSWORD=horariosUser_dev
ENV POSTGRES_DB=horarioshub

COPY dump.sql /docker-entrypoint-initdb.d/01-dump.sql