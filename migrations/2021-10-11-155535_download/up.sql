create table words (
    id   integer  primary key autoincrement,
    word text     not null unique
);

create table one_grams (
    id       integer primary key autoincrement,
    word1_id integer not null,
    score    integer not null,

    constraint fk_one_grams_word1_id
        foreign key (word1_id)
        references words(id)
        on delete cascade
);

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

create table fetched_files (
    n   integer not null,
    idx integer not null,

    primary key(n, idx)
);
