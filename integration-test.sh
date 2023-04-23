#/bin/bash


# Shapes collection is public
# Letters collection is owner access only

# Account without any role
NO_ROLE_CLIENT=inttest_norole
NO_ROLE_SECRET=qcJT4PLAldiC8owRsDhgtiCoMzuqlWId

# Account with role Collection Administrator
COLADMIN_CLIENT=inttest_coladmin
COLADMIN_SECRET=q3RRoqv6tQNP8PRVJq0WdQOU1WKmbU6X

# Account with role Editor for Shapes collection
SHAPES_EDITOR_CLIENT=inttest_shapes_editor
SHAPES_EDITOR_SECRET=Ha7hcGzlHHYQc0rMS9vtlaecDHunTG8I

# Account with role Reader for Shapes collection
SHAPES_READER_CLIENT=inttest_shapes_reader
SHAPES_READER_SECRET=hI523HzLvNmg8WDn4Dd7DY1NmAIV0KtK

# Account with roles for Letters collection
LETTERS_ALPACA_USER_CLIENT=inttest_letters_alpaca
LETTERS_ALPACA_USER_SECRET=xcZrWkLvJo0wcfjzWIefnYn8yNNRu7Dj

# Another account with roles for Letters collection
LETTERS_BEAR_USER_CLIENT=inttest_letters_bear
LETTERS_BEAR_USER_SECRET=pwRV5k0IzscIJ34WKMndamsyefWBV0pD

OIDCTOKEN=""
API=http://localhost:3000/api

function authorize_client {
  OIDCTOKEN=$(curl --silent --location --request POST 'http://localhost:8101/realms/folivafy/protocol/openid-connect/token' \
    --header 'Content-Type: application/x-www-form-urlencoded' \
    --data-urlencode "client_id=$1" \
    --data-urlencode "client_secret=$2" \
    --data-urlencode 'scope=openid' \
    --data-urlencode 'grant_type=client_credentials' | jq -r '.access_token')
}


cargo build
export DATABASE_URL=postgresql://inttest_role:inttest_pwd@db/inttest
./target/debug/migration

# Start binary in background
rm nohup.out
nohup /bin/bash -c "RUST_LOG=debug FOLIVAFY_DATABASE=$DATABASE_URL ./target/debug/folivafy" &
serverPID=$!
sleep 0.15


#####################################################
##
##  Administrative paths
##
#####################################################


echo "- Access denied for user without coladmin role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
if [ "$RESP" != "Unauthorized" ]
then
      echo "Failure: user is allowed to list collections!"
fi


echo "- Cannot create shapes collection with user $NO_ROLE_CLIENT"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"name": "shapes","title": "Shapes","oao": false}' \
  $API/collections)
if [ "$RESP" == "Collection shapes created" ]
then
      echo "Failure: user is allowed to create a collection!"
fi


echo "- Can list collections, list is empty"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
if [ "$RESP" != '{"limit":50,"offset":0,"total":0,"items":[]}' ]
then
      echo "Failure: user is not allowed to list collections!\n$RESP"
fi


echo "- Can create shapes collection"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"name": "shapes","title": "Shapes","oao": false}' \
  $API/collections)
if [ "$RESP" != "Collection shapes created" ]
then
      echo "Failure: user is not allowed to create a collection!"
fi


echo "- Cannot create shapes collection twice"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"name": "shapes","title": "Shapes","oao": false}' \
  $API/collections)
if [ "$RESP" != "Duplicate collection name" ]
then
      echo "Failure: user is not allowed to create a collection!"
fi


echo "- Can create letters collection"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"name": "letters","title": "Letters","oao": true}' \
  $API/collections)
if [ "$RESP" != "Collection letters created" ]
then
      echo "Failure: user is not allowed to create letters collection!"
fi


echo "- Can list collections"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "2" ]
then
      echo "Failure: list of collections incomplete!\n$RESP"
fi

#####################################################
##
##  Public shapes collection
##
#####################################################

echo "- User can create rectangle shape document"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ea25fa9d-4650-41ae-a1fa-00bd226b648f","f": {"title": "Rectangle", "price": 14}}' \
  $API/collections/shapes)
if [ "$RESP" != "Document saved" ]
then
      echo "Failure: user is not allowed to save rectangle document!\n$RESP"
fi


echo "- User cannot create rectangle shape document twice"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ea25fa9d-4650-41ae-a1fa-00bd226b648f","f": {"title": "Rectangle", "price": 14}}' \
  $API/collections/shapes)
if [ "$RESP" != "Duplicate document" ]
then
      echo "Failure: duplicate rectangle document!\n$RESP"
fi


echo "- User can create circle shape document"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "1dec98bb-564e-4e40-81b9-e9aa5ab098f6","f": {"title": "Circle", "price": 9}}' \
  $API/collections/shapes)
if [ "$RESP" != "Document saved" ]
then
      echo "Failure: user is not allowed to save circle document!\n$RESP"
fi


echo "- Access denied for user $NO_ROLE without shapes reader role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" != "Unauthorized" ]
then
      echo "Failure: user is allowed to list documents!\n$RESP"
fi


echo "- Access denied for user $SHAPES_EDITOR_CLIENT without shapes reader role"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" != "Unauthorized" ]
then
      echo "Failure: user is allowed to list documents!\n$RESP"
fi


echo "- Can list shapes"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" == "Unauthorized" ]
then
      echo "Failure: user is not allowed to list documents!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "2" ]
then
      echo "Failure: list of documents incomplete!\n$RESP"
fi


#####################################################
##
##  Owner access only collection
##
#####################################################


echo "- User 1 can create a letter document"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ff901d16-a533-4ad7-9e75-d69407440804","f": {"title": "Alpaca letter 1", "content": "foo"}}' \
  $API/collections/letters)
if [ "$RESP" != "Document saved" ]
then
      echo "Failure: user is not allowed to save Alpaca letter 1 document!\n$RESP"
fi


echo "- User 2 can create a letter document"
authorize_client $LETTERS_BEAR_USER_CLIENT $LETTERS_BEAR_USER_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "fc3b8fce-cac3-4dd5-92b9-11d5963b9a89","f": {"title": "Bear letter 1", "content": "baz"}}' \
  $API/collections/letters)
if [ "$RESP" != "Document saved" ]
then
      echo "Failure: user is not allowed to save Bear letter 1 document!\n$RESP"
fi


echo "- User 1 can create second letter document"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "44c64580-a795-4d56-a69d-140f726153f8","f": {"title": "Alpaca letter 2", "content": "bar"}}' \
  $API/collections/letters)
if [ "$RESP" != "Document saved" ]
then
      echo "Failure: user is not allowed to save Alpaca letter 2 document!\n$RESP"
fi


echo "- Access denied for user $NO_ROLE without letters reader role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" != "Unauthorized" ]
then
      echo "Failure: user is allowed to list letters!\n$RESP"
fi


echo "- User 1 can list its letters"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" == "Unauthorized" ]
then
      echo "Failure: user is not allowed to list letters!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "2" ]
then
      echo "Failure: list of documents incomplete!\n$RESP"
fi


echo "- User 2 can list its letters"
authorize_client $LETTERS_BEAR_USER_CLIENT $LETTERS_BEAR_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" == "Unauthorized" ]
then
      echo "Failure: user is not allowed to list letters!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "1" ]
then
      echo "Failure: list of documents incomplete!\n$RESP"
fi


kill $serverPID
