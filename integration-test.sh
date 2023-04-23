#/bin/bash



NO_ROLE_CLIENT=inttest_norole
NO_ROLE_SECRET=qcJT4PLAldiC8owRsDhgtiCoMzuqlWId

COLADMIN_CLIENT=inttest_coladmin
COLADMIN_SECRET=q3RRoqv6tQNP8PRVJq0WdQOU1WKmbU6X

SHAPES_EDITOR_CLIENT=inttest_shapes_editor
SHAPES_EDITOR_SECRET=Ha7hcGzlHHYQc0rMS9vtlaecDHunTG8I

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


echo "- Access denied for user without coladmin role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
if [ "$RESP" != "Unauthorized" ]
then
      echo "Failure: user is allowed to list collections!"
fi


echo "- Cannot create shapes collection"
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


kill $serverPID
