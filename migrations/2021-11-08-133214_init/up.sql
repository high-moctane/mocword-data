create table if not exists one_grams (
    id    integer not null primary key,
    word  text    not null,
    score integer not null
);

create table if not exists two_grams (
    word1 integer not null,
    word2 integer not null,
    score integer not null,

    primary key (word1, word2)
);

create table if not exists three_grams (
    word1 integer not null,
    word2 integer not null,
    word3 integer not null,
    score integer not null,

    primary key (word1, word2, word3)
);

create table if not exists four_grams (
    word1 integer not null,
    word2 integer not null,
    word3 integer not null,
    word4 integer not null,
    score integer not null,

    primary key (word1, word2, word3, word4)
);

create table if not exists five_grams (
    word1 integer not null,
    word2 integer not null,
    word3 integer not null,
    word4 integer not null,
    word5 integer not null,
    score integer not null,

    primary key (word1, word2, word3, word4, word5)
);

create table if not exists fetched_files (
    n   integer not null,
    idx integer not null,

    primary key (n, idx)
);
