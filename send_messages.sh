#!/bin/bash

for ((i=100; i<=200; i++))
do
    # Construct the message with the iteration number
    message="{\"msg\":\"Hello World - Iteration $i\"}"

    curl -X POST http://localhost:8000/ -H "Content-Type: application/json" -d "$message"

    echo
done
