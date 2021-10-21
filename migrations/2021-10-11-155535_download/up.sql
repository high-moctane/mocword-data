create table words (
    id   integer  primary key autoincrement,
    word text     not null unique
);

create table one_grams (
    id       integer primary key autoincrement,
    word1_id integer not null unique,
    score    integer not null,

    constraint fk_one_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade
);

create index idx_one_grams_score on one_grams(score);

create table two_grams (
    id       integer primary key autoincrement,
    word1_id integer not null,
    word2_id integer not null,
    score    integer not null,

    constraint fk_two_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade,
    constraint fk_two_grams_word2_id
        foreign key (word2_id)
        references words(id)
        on delete cascade
);

create unique index idx_two_grams on two_grams(word1_id, word2_id);
create index idx_two_grams_score on two_grams(score);

create table three_grams (
    id       integer primary key autoincrement,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    score    integer not null,

    constraint fk_three_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade,
    constraint fk_three_grams_word2_id
        foreign key (word2_id)
        references words(id)
        on delete cascade,
    constraint fk_three_grams_word3_id
        foreign key (word3_id)
        references words(id)
        on delete cascade
);

create unique index idx_three_grams on three_grams(word1_id, word2_id, word3_id);
create index idx_three_grams_score on three_grams(score);

create table four_grams (
    id       integer primary key autoincrement,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    word4_id integer not null,
    score    integer not null,

    constraint fk_four_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade,
    constraint fk_four_grams_word2_id
        foreign key (word2_id)
        references words(id)
        on delete cascade,
    constraint fk_four_grams_word3_id
        foreign key (word3_id)
        references words(id)
        on delete cascade,
    constraint fk_four_grams_word4_id
        foreign key (word4_id)
        references words(id)
        on delete cascade
);

create unique index idx_four_grams on four_grams(word1_id, word2_id, word3_id, word4_id);
create index idx_four_grams_score on four_grams(score);

create table five_grams (
    id       integer primary key autoincrement,
    word1_id integer not null,
    word2_id integer not null,
    word3_id integer not null,
    word4_id integer not null,
    word5_id integer not null,
    score    integer not null,

    constraint fk_five_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade,
    constraint fk_five_grams_word2_id
        foreign key (word2_id)
        references words(id)
        on delete cascade,
    constraint fk_five_grams_word3_id
        foreign key (word3_id)
        references words(id)
        on delete cascade,
    constraint fk_five_grams_word4_id
        foreign key (word4_id)
        references words(id)
        on delete cascade,
    constraint fk_five_grams_word5_id
        foreign key (word5_id)
        references words(id)
        on delete cascade
);

create unique index idx_five_grams on five_grams(word1_id, word2_id, word3_id, word4_id, word5_id);
create index idx_five_grams_score on four_grams(score);

create table fetched_files (
    n   integer not null,
    idx integer not null,

    primary key(n, idx)
);
