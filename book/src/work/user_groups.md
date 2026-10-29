# Adding and Removing One Group

A user's groups can be changed one at a time. You do not need to send the whole list.

```
PUT    /auth/v1/users/{id}/groups/{group}     add this group
DELETE /auth/v1/users/{id}/groups/{group}     remove this group
```

Both answer `200` with the user, including all of the groups it has afterwards.

```bash
curl -X PUT    -H "Authorization: API-Key $KEY" https://iam.example.com/auth/v1/users/$ID/groups/staff
curl -X DELETE -H "Authorization: API-Key $KEY" https://iam.example.com/auth/v1/users/$ID/groups/staff
```

- **Only this group changes.** Every other group of the user stays as it is.
- **Repeating is safe.** Adding a group the user already has, or removing one it does not have, changes nothing and
  answers `200`.
- **The group must exist** to be added (`404` otherwise). A user that does not exist is a `404` too. System users
  (see [System Users and the Audit Log](audit_log.md)) cannot be changed (`403`).
- **Changes at the same moment do not overwrite each other.** Eight scripts adding eight different groups to one user
  at once end with all eight groups. See below for how.
- **Permissions** are the ones of editing a user: `rauthy_admin`, an API key with `Users` `Update`, or a
  [delegated group admin](group_admins.md), who can only add and remove groups they manage and never touches roles.
- The change is recorded in the [audit log](audit_log.md) as an update of the user, with the path as the detail.
  Sessions and tokens that are already issued keep the groups they were issued with, like with any other user change.

## Why the API Used an Array

The groups of a user are not rows in a link table and they are not ids. The `users` table has one text column,
`groups`, that holds the **names** of the groups, separated by commas, for example `staff,finance`. Roles are stored the
same way. This is upstream's design: the user is read as one row (and one cache entry) at every login, and the groups
for the token come out of that row without a join.

The consequence is that `PUT /users/{id}` and `PATCH /users/{id}` with `groups` write the **whole** column, so a caller
has to read the list, change it and send it all back. Two callers doing this at the same time can lose each other's
change, and a script that only wants to add one group needs to know all the others. The two routes above fix that for
groups.

## How It Stays Correct

The column is still one string, so the change is a compare and swap in SQL:

```sql
UPDATE users SET groups = <new list> WHERE id = <id> AND groups IS <the list that was read>
```

If another change got in first, no row matches, the change reads the user again and applies itself on top, up to ten
times. On one node the changes also queue up, so this is rare. It works the same on Hiqlite and Postgres. Nothing in the
schema changed and there is no migration.

`PUT` / `PATCH /users/{id}` still work as before. Renaming or deleting a group is unchanged.
