# The calendar for the company edition (`advent serve`): the engine image plus
# this repo's sealed files. The release key and other secrets are given to the
# container when it runs, never built into the image.
#
#   docker build --build-arg ENGINE=$(grep '^engine:' advent.yaml | tr -d '" ' | cut -d: -f2) -t my-calendar .
#
# .dockerignore lets only sealed and public files in: puzzles, answers and the
# editors key stay out even when building from an author's working copy.

ARG ENGINE_IMAGE=ghcr.io/ashdevfr/advent
ARG ENGINE
FROM ${ENGINE_IMAGE}:${ENGINE}
COPY . /event
