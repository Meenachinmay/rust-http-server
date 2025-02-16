#!/bin/bash

# First create a user and capture the response
echo "Creating new user..."
response=$(curl -s -X POST \
  http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secretpassword123"
  }')

# Extract the ID from the response using jq (if you have it installed)
# You'll need to install jq: 'sudo apt-get install jq' on Ubuntu/Debian
user_id=$(echo $response | jq -r '.id')

echo "Created user with ID: $user_id"

# Get the user we just created
echo "Fetching user details..."
curl -X GET \
  "http://localhost:8080/users/$user_id" \
  -H "Accept: application/json"