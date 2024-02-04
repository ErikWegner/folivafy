#!/usr/bin/env bash

# Account with access to mail queue
APICLIENT_CLIENT=test_mailqueue
APICLIENT_SECRET=wCgIst8BGjgwxcfvM19T6w2wzeYdWPVA

OIDCTOKEN=""
API=http://localhost:3002/api

function authorize_client {
  OIDCTOKEN=$(curl --silent --location --request POST 'http://localhost:8101/realms/folivafy/protocol/openid-connect/token' \
    --header 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode "client_id=$1" \
    --data-urlencode "client_secret=$2" \
    --data-urlencode 'scope=openid' \
    --data-urlencode 'grant_type=client_credentials' | jq -r '.access_token')
}

authorize_client $APICLIENT_CLIENT $APICLIENT_SECRET
UUID=$(python3 -c 'import uuid; print(uuid.uuid4())')
curl -i \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "'${UUID}'","f": {"title": "Mail", "to": "recp@example.local", "subject": "A test mail", "body_text": "body text", "body_html": "body html", "status": "Pending"}}' \
  $API/collections/folivafy-mail && echo -e "\n\nCreated ${UUID}"

echo "Press any key to continue..."
read -s -n 1

authorize_client $APICLIENT_CLIENT $APICLIENT_SECRET
curl -i --header "Authorization: Bearer $OIDCTOKEN" $API/collections/folivafy-mail?extraFields=status
