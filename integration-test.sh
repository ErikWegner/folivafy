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

# Account with role Editor for Fluids collection
FLUIDS_EDITOR_CLIENT=inttest_fluids_editor
FLUIDS_EDITOR_SECRET=Zjn8HoTjeedDtDJsYpIk6ZtCdhogHd2J

# Account with role Reader for Shapes collection
SHAPES_READER_CLIENT=inttest_shapes_reader
SHAPES_READER_SECRET=hI523HzLvNmg8WDn4Dd7DY1NmAIV0KtK

# Account with role Reader for Shapes collection
SHAPES_READER_OTHER_CLIENT=inttest_shapes_reader_other
SHAPES_READER_OTHER_SECRET=ZcSYrGhXVpRRtLUmGn7CQpcMw3sVw2PT

# Account with roles for Letters collection
LETTERS_ALPACA_USER_CLIENT=inttest_letters_alpaca
LETTERS_ALPACA_USER_SECRET=xcZrWkLvJo0wcfjzWIefnYn8yNNRu7Dj

# Another account with roles for Letters collection
LETTERS_BEAR_USER_CLIENT=inttest_letters_bear
LETTERS_BEAR_USER_SECRET=pwRV5k0IzscIJ34WKMndamsyefWBV0pD

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

RED='\033[0;31m'
NC='\033[0m' # No Color

cargo build --package folivafy --package migration
if [ $? -ne 0 ]; then
  echo “Error: Failed to build”
  exit 1
fi

export USERDATA_CLIENT_ID=clientname
export USERDATA_CLIENT_SECRET=clientsecret
export USERDATA_TOKEN_URL=https://identity/token/url
export USERDATA_USERINFO_URL=https://identity/users/{id}
export DATABASE_URL=postgresql://inttest_role:inttest_pwd@db/inttest
./target/debug/migration

# Start binary in background
rm nohup.out
nohup /bin/bash -c "RUST_LOG=debug FOLIVAFY_DATABASE=$DATABASE_URL ./target/debug/folivafy" &
serverPID=$!
sleep 0.5


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
      echo -e "${RED}Failure:${NC} user is allowed to list collections!"
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
      echo -e "${RED}Failure:${NC} user is allowed to create a collection!"
fi


echo "- Can list collections, list is empty (except mail)"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
if [ "$RESP" != '{"limit":50,"offset":0,"total":1,"items":[{"name":"folivafy-mail","title":"Folivafy mail","oao":true,"locked":false}]}' ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list collections!\n$RESP"
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
      echo -e "${RED}Failure:${NC} user is not allowed to create a collection!"
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
      echo -e "${RED}Failure:${NC} user is not allowed to create a collection!"
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
      echo -e "${RED}Failure:${NC} user is not allowed to create letters collection!"
fi


echo "- Can list collections"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections)
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "3" ]
then
      echo -e "${RED}Failure:${NC} list of collections incomplete!\n$RESP"
fi


echo "- Can create fluids collection"
authorize_client $COLADMIN_CLIENT $COLADMIN_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"name": "fluids","title": "Fluids","oao": false}' \
  $API/collections)
if [ "$RESP" != "Collection fluids created" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to create fluids collection!"
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
      echo -e "${RED}Failure:${NC} user is not allowed to save rectangle document!\n$RESP"
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
      echo -e "${RED}Failure:${NC} duplicate rectangle document!\n$RESP"
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
      echo -e "${RED}Failure:${NC} user is not allowed to save circle document!\n$RESP"
fi


echo "- Access denied for user $NO_ROLE without shapes reader role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" != "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is allowed to list documents!\n$RESP"
fi


echo "- Access denied for user $SHAPES_EDITOR_CLIENT without shapes reader role"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" != "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is allowed to list documents!\n$RESP"
fi


echo "- Can list shapes"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list documents!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "2" ]
then
      echo -e "${RED}Failure:${NC} list of documents incomplete!\n$RESP"
fi


echo "- Can list shapes with additional fields"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes?extraFields=price)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list documents!\n$RESP"
fi
FIELDS=$(echo $RESP | jq '.items[].f.title, .items[].f.price' | jq -s -r 'join(" ")')
if [ "$FIELDS" != "Rectangle Circle 14 9" ]
then
      echo -e "${RED}Failure:${NC} list of documents is missing fields!\n$FIELDS\n$RESP"
fi


echo "- Can list shapes with exact title match"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes?exactTitle=Rectangle)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list documents!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
if [ "$TOTAL" != "1" ]
then
      echo -e "${RED}Failure:${NC} list of filtered documents does not match!\n$RESP"
fi
TITLE=$(echo $RESP | jq -r '.items[0].f.title')
if [ "$TITLE" != "Rectangle" ]
then
      echo -e "${RED}Failure:${NC} document title does not match!\n$RESP"
fi


echo "- User can create water fluid document"
authorize_client $FLUIDS_EDITOR_CLIENT $FLUIDS_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "702562c8-8017-4b95-9c07-dfaceb5496ed","f": {"title": "Water"}}' \
  $API/collections/fluids)
if [ "$RESP" != "Document saved" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to save water document!\n$RESP"
fi

# sorting with sub fields


echo "- User can create triangle shape document"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "9097a77f-eb07-4110-81ff-fbefdef25cd5","f": {"title": "Triangle", "geo": { "edges": 3}}}' \
  $API/collections/shapes)
if [ "$RESP" != "Document saved" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to save triangle document!\n$RESP"
fi


echo "- User can create hexagon shape document"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request POST \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "be7c1d84-e27d-42a0-8abd-54a1b2c17e36","f": {"title": "Hexagon", "geo": { "edges": 6}}}' \
  $API/collections/shapes)
if [ "$RESP" != "Document saved" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to save hexagon document!\n$RESP"
fi


echo "- Can list shapes sorted asc by fields"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes?sort=geo.edges\%2B\&extraFields=geo)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list documents!\n$RESP"
fi
FIELDS=$(echo $RESP | jq '[.items[] | {t: .f.title, g: .f.geo.edges}][] | .t, .g' | jq -s -r 'join(" ")')
if [ "$FIELDS" != "Triangle 3 Hexagon 6 Rectangle  Circle " ]
then
      echo -e "${RED}Failure:${NC} list of documents with sort fields failed!\n$FIELDS\n$RESP"
fi


echo "- Can list shapes sorted desc by fields"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes?sort=geo.edges-\&extraFields=geo)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list documents!\n$RESP"
fi
FIELDS=$(echo $RESP | jq '[.items[] | {t: .f.title, g: .f.geo.edges}][] | .t, .g' | jq -s -r 'join(" ")')
if [ "$FIELDS" != "Rectangle  Circle  Hexagon 6 Triangle 3" ]
then
      echo -e "${RED}Failure:${NC} list of documents with sort fields failed!\n$FIELDS\n$RESP"
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
      echo -e "${RED}Failure:${NC} user is not allowed to save Alpaca letter 1 document!\n$RESP"
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
      echo -e "${RED}Failure:${NC} user is not allowed to save Bear letter 1 document!\n$RESP"
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
      echo -e "${RED}Failure:${NC} user is not allowed to save Alpaca letter 2 document!\n$RESP"
fi


echo "- Access denied for user $NO_ROLE without letters reader role"
authorize_client $NO_ROLE_CLIENT $NO_ROLE_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" != "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is allowed to list letters!\n$RESP"
fi


echo "- User 1 can list its letters"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list letters!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
LENGTH=$(echo $RESP | jq -r '.items | length')
if [ "$TOTAL" != "2" ] || [ "$LENGTH" != "2" ]
then
      echo -e "${RED}Failure:${NC} list of documents incomplete!\n$RESP"
fi


echo "- User 2 can list its letters"
authorize_client $LETTERS_BEAR_USER_CLIENT $LETTERS_BEAR_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list letters!\n$RESP"
fi
TOTAL=$(echo $RESP | jq -r '.total')
LENGTH=$(echo $RESP | jq -r '.items | length')
if [ "$TOTAL" != "1" ] || [ "$LENGTH" != "1" ]
then
      echo -e "${RED}Failure:${NC} list of documents incomplete!\n$RESP"
fi


echo "- User 1 can list its letters with sorting"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API'/collections/letters?sort=content-,title-&extraFields=content')
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to list letters!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.items | map(.f.content) | join(",")')
if [ "$CONTENT" != "foo,bar" ]
then
      echo -e "GOT $CONTENT!\n$RESP"
fi


#####################################################
##
##  Document read access
##
#####################################################


echo "- User can read rectangle"
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes/ea25fa9d-4650-41ae-a1fa-00bd226b648f)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read rectangle!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.f.price')
if [ "$CONTENT" != "14" ]
then
      echo -e "${RED}Failure:${NC} cannot read document!\n$RESP"
fi


echo "- Other user can read rectangle"
authorize_client $SHAPES_READER_OTHER_CLIENT $SHAPES_READER_OTHER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes/ea25fa9d-4650-41ae-a1fa-00bd226b648f)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} other user is not allowed to read rectangle!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.f.price')
if [ "$CONTENT" != "14" ]
then
      echo -e "${RED}Failure:${NC} cannot read document!\n$RESP"
fi


echo "- User 1 can retrieve its letter"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters/ff901d16-a533-4ad7-9e75-d69407440804)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read alpaca letter 1!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.f.content')
if [ "$CONTENT" != "foo" ]
then
      echo -e "${RED}Failure:${NC} alpaca letter 1 content (1)!\n$RESP"
fi


echo "- User 2 cannot retrieve other users letter"
authorize_client $LETTERS_BEAR_USER_CLIENT $LETTERS_BEAR_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters/ff901d16-a533-4ad7-9e75-d69407440804)
if [ "$RESP" != "Document ff901d16-a533-4ad7-9e75-d69407440804 not found" ]
then
      echo -e "${RED}Failure:${NC} user is allowed to read alpaca letter 1!\n$RESP"
fi


echo "- Fluid user can read water"
authorize_client $FLUIDS_EDITOR_CLIENT $FLUIDS_EDITOR_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/fluids/702562c8-8017-4b95-9c07-dfaceb5496ed)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} other user is not allowed to read water!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.f.title')
if [ "$CONTENT" != "Water" ]
then
      echo -e "${RED}Failure:${NC} cannot read water document!\n$RESP"
fi


echo "- Fluid user can read rectangle"
authorize_client $FLUIDS_EDITOR_CLIENT $FLUIDS_EDITOR_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes/ea25fa9d-4650-41ae-a1fa-00bd226b648f)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read rectangle!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -r '.f.price')
if [ "$CONTENT" != "14" ]
then
      echo -e "${RED}Failure:${NC} cannot read document!\n$RESP"
fi


echo "- No user can read rectangle accessed through wrong collection"
authorize_client $FLUIDS_EDITOR_CLIENT $FLUIDS_EDITOR_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/fluids/ea25fa9d-4650-41ae-a1fa-00bd226b648f)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read rectangle!\n$RESP"
fi
if [ "$RESP" != "Document ea25fa9d-4650-41ae-a1fa-00bd226b648f not found" ]
then
      echo -e "${RED}Failure:${NC} document was available!\n$RESP"
fi


#####################################################
##
##  Document update access
##
#####################################################


echo "- User can update rectangle"
authorize_client $SHAPES_EDITOR_CLIENT $SHAPES_EDITOR_SECRET
RESP=$(curl --silent \
  --request PUT \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ea25fa9d-4650-41ae-a1fa-00bd226b648f","f": {"title": "Square", "area": 3}}' \
  $API/collections/shapes)
if [ "$RESP" != "Document updated" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to save rectangle document!\n$RESP"
fi
authorize_client $SHAPES_READER_CLIENT $SHAPES_READER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/shapes/ea25fa9d-4650-41ae-a1fa-00bd226b648f)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read square!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -c -r '.|=(.e[]|=(del(.ts)))')
if [ "$CONTENT" != '{"id":"ea25fa9d-4650-41ae-a1fa-00bd226b648f","f":{"area":3,"title":"Square"},"e":[{"id":9,"category":1,"e":{"user":{"id":"98ebb628-4a46-4274-a9f0-eb7c6f385540","name":"service-account-inttest_shapes_editor"}}},{"id":1,"category":1,"e":{"new":true,"user":{"id":"98ebb628-4a46-4274-a9f0-eb7c6f385540","name":"service-account-inttest_shapes_editor"}}}]}' ]
then
      echo -e "${RED}Failure:${NC} square content!\n$RESP\n$CONTENT"
fi


echo "- Alpaca can update letter"
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent \
  --request PUT \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ff901d16-a533-4ad7-9e75-d69407440804","f": {"title": "Alpaca letter 1/b", "content": "FooFoo"}}' \
  $API/collections/letters)
if [ "$RESP" != "Document updated" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to update Alpaca letter 1!\n$RESP"
fi
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters/ff901d16-a533-4ad7-9e75-d69407440804)
if [ "$RESP" == "Unauthorized" ]
then
      echo -e "${RED}Failure:${NC} user is not allowed to read Alpaca letter 1!\n$RESP"
fi
CONTENT=$(echo $RESP | jq -c -r '.|=(.e[]|=(del(.ts)))')
if [ "$CONTENT" != '{"id":"ff901d16-a533-4ad7-9e75-d69407440804","f":{"content":"FooFoo","title":"Alpaca letter 1/b"},"e":[{"id":10,"category":1,"e":{"user":{"id":"f299112d-9110-48fc-8769-9d5bab6e37fb","name":"service-account-inttest_letters_alpaca"}}},{"id":6,"category":1,"e":{"new":true,"user":{"id":"f299112d-9110-48fc-8769-9d5bab6e37fb","name":"service-account-inttest_letters_alpaca"}}}]}' ]
then
      echo -e "${RED}Failure:${NC} Alpaca letter 1 content (2)!\n$RESP\n$CONTENT"
fi


echo "- Bear cannot update Alpaca letter"
authorize_client $LETTERS_BEAR_USER_CLIENT $LETTERS_BEAR_USER_SECRET
RESP=$(curl --silent \
  --request PUT \
  --header "Authorization: Bearer $OIDCTOKEN" \
  --header "Content-Type: application/json" \
  --data '{"id": "ff901d16-a533-4ad7-9e75-d69407440804","f": {"title": "Alpaca letter 1/b", "content": "FooFoo"}}' \
  $API/collections/letters)
if [ "$RESP" == "Document updated" ]
then
      echo -e "${RED}Failure:${NC} bear is allowed to update Alpaca letter 1!\n$RESP"
fi
authorize_client $LETTERS_ALPACA_USER_CLIENT $LETTERS_ALPACA_USER_SECRET
RESP=$(curl --silent --header "Authorization: Bearer $OIDCTOKEN" $API/collections/letters/ff901d16-a533-4ad7-9e75-d69407440804)
CONTENT=$(echo $RESP | jq -c -r '.|=(.e[]|=(del(.ts)))')
if [ "$CONTENT" != '{"id":"ff901d16-a533-4ad7-9e75-d69407440804","f":{"content":"FooFoo","title":"Alpaca letter 1/b"},"e":[{"id":10,"category":1,"e":{"user":{"id":"f299112d-9110-48fc-8769-9d5bab6e37fb","name":"service-account-inttest_letters_alpaca"}}},{"id":6,"category":1,"e":{"new":true,"user":{"id":"f299112d-9110-48fc-8769-9d5bab6e37fb","name":"service-account-inttest_letters_alpaca"}}}]}' ]
then
      echo -e "${RED}Failure:${NC} Alpaca letter 1 content (3)!\n$RESP\n$CONTENT"
fi


kill $serverPID
