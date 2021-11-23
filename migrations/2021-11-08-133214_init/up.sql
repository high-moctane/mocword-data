create sequence seq;

create table if not exists one_grams (
    id    int    not null default (next value for seq) primary key,
    word  text   not null,
    score bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists two_grams (
    word1 int    not null,
    word2 int    not null,
    score bigint not null,

    primary key (word1, word2)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists three_grams (
    word1 int    not null,
    word2 int    not null,
    word3 int    not null,
    score bigint not null,

    primary key (word1, word2, word3)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists four_grams (
    word1 int    not null,
    word2 int    not null,
    word3 int    not null,
    word4 int    not null,
    score bigint not null,

    primary key (word1, word2, word3, word4)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists five_grams (
    word1 int    not null,
    word2 int    not null,
    word3 int    not null,
    word4 int    not null,
    word5 int    not null,
    score bigint not null,

    primary key (word1, word2, word3, word4, word5)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists fetched_files (
    n        int not null,
    idx      int not null,

    primary key (n, idx)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';
