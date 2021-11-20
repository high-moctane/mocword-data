create sequence seq;

create table if not exists one_grams (
    id    bigint not null default (next value for seq) primary key,
    word  text   not null,
    score bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create sequence seq_two_grams;

create table if not exists two_grams (
    id        bigint not null default (next value for seq) primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists three_grams (
    id        bigint not null default (next value for seq) primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists four_grams (
    id        bigint not null default (next value for seq) primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists five_grams (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists fetched_files (
    n        bigint not null,
    idx      bigint not null,

    primary key (n, idx)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';
