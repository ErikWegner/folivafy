# Folivafy

A PostgreSQL backed document oriented database with document owner access control.

## Details

This service manages _documents_ in _collections_. It provides administrative
endpoints to create new collections.

A collection can be set to be _owner access only_ (`oao`). If set, the documents in the
collection can only be read and edited by their owners and an additional group
of administrators.

Within a collection, documents can be stored and retrieved.

## Permissions

To administer collections, a user needs the role `A_FOLIVAFY_COLLECTION_EDITOR`.

To have read access to a collection, a role with the name of the collection is
checked: `C_<NAME-OF-COLLECTION>_READER`. A collection with `oao` set to `false`
lets the user see all documents. If `oao` is set to `true`, only documents where
the user is the owner are readable.

To create and edit documents in a collection, a role with the name
`C_<NAME-OF-COLLECTION>_EDITOR` is checked.

To see all documents regardless of the `oao` setting, a user can be assigned the
role `C_<NAME-OF-COLLECTION>_ADMIN`.

## Authentication

A [Keycloak](https://keycloak.org) configuration is contained in this repository.

Some users are already set up:

User _coladmin_ can administer collections.

To update the file from a running keycloak instance, use these commands:

```bash
docker exec -it folivafy_devcontainer-keycloak-1 /bin/bash -c "/opt/keycloak/bin/kc.sh export --file /opt/keycloak/dev_realm.json --realm folivafy --users same_file"
docker cp folivafy_devcontainer-keycloak-1:/opt/keycloak/dev_realm.json dev_realm.json
```

## SEA ORM Entity Generation

```bash
cargo install sea-orm-cli
export DATABASE_URL=postgresql://postgres:postgres@db/postgres
sea-orm-cli generate entity -o entity/src
```

## Integration tests

```bash
cat integration-test.sql | docker exec -i folivafy_devcontainer-db-1 psql -U postgres postgres
docker exec -it --user $(id -u):$(id -g) folivafy_devcontainer-app-1 /bin/bash -c "cd /workspaces/folivafy ; ./integration-test.sh"
```
