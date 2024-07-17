#!/usr/bin/env bash
# Bash script that uploads all files on ./downloads to r2 instance using wrlanger

files=$(ls ./downloads)
i=0
for file in $files
do 
    echo "Uploading $file"
    wrangler r2 object put nosesisaid-cdn/$file --file=downloads/$file
    i=$((i+1))
done
echo "Uploaded $i files"