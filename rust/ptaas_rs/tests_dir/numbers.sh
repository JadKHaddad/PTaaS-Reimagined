#!/bin/bash

i=1
while [ "$i" -le 3 ]
do
    echo "$i"
    ((i++))
    sleep 1
done