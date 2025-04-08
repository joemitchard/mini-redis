#!/bin/bash

redis-cli ping &
redis-cli ping &
echo -e "ping\nping" | redis-cli
