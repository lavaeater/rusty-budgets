# Migrating to PostgreSQL

Currently we have used the joydb crate to enable skipping the whole creating and migrating of schemas.

However, storing the entire database in the repo as an
as of now 7.85 MB json file is perhaps not the best strategy moving forward.

It would behoove us to migrate to using PostgreSQL - we have it setup in the local docker network, we know how it works, it has indexes, it is fast, the support using sqlx is fantastic. I would love if there existed a really simple ORM-mapping tool to create queries without tons of string manipulation, but they are as of now so unwieldly, in my mind. 