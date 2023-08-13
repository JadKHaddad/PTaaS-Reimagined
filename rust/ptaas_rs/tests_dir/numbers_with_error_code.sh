#!/bin/bash

i=1
while [ "$i" -le 1 ]
do
    echo "$i"
    ((i++))
    sleep 1
done

echo "Error message" >&2

exit 1