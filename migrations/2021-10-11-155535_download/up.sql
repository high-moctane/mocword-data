pragma foreign_keys = ON;

create table words (
    id   int  not null primary key,
    word text not null unique
);

create table one_grams (
    id       int not null primary key,
    word1_id int not null unique,

    constraint fk_one_grams_word1_id foreign key (word1_id) references words(id)
);

create table one_gram_entries (
    id           int not null primary key,
    one_gram_id  int not null,
    year         int not null,
    match_count  int not null,
    volume_count int not null,

    constraint fk_one_gram_entries_one_gram_id foreign key (one_gram_id) references one_grams(id)
);

create unique index idx_one_gram_entries on one_gram_entries(one_gram_id, year);

create table two_grams (
    id       int not null primary key,
    word1_id int not null,
    word2_id int not null,

    constraint fk_two_grams_word1_id foreign key (word1_id) references words(id),
    constraint fk_two_grams_word2_id foreign key (word2_id) references words(id)
);

create unique index idx_two_grams on two_grams(word1_id, word2_id);

create table two_gram_entries (
    id           int not null primary key,
    two_gram_id  int not null,
    year         int not null,
    match_count  int not null,
    volume_count int not null,

    constraint fk_two_gram_entries_two_gram_id foreign key (two_gram_id) references two_grams(id)
);

create unique index idx_two_gram_entries on two_gram_entries(two_gram_id, year);

create table three_grams (
    id       int not null primary key,
    word1_id int not null,
    word2_id int not null,
    word3_id int not null,

    constraint fk_three_grams_word1_id foreign key (word1_id) references words(id),
    constraint fk_three_grams_word2_id foreign key (word2_id) references words(id),
    constraint fk_three_grams_word3_id foreign key (word3_id) references words(id)
);

create unique index idx_three_grams on three_grams(word1_id, word2_id, word3_id);

create table three_gram_entries (
    id            int not null primary key,
    three_gram_id int not null,
    year          int not null,
    match_count   int not null,
    volume_count  int not null,

    constraint fk_three_gram_entries_three_gram_id foreign key (three_gram_id) references three_grams(id)
);

create unique index idx_three_gram_entries on three_gram_entries(three_gram_id, year);

create table four_grams (
    id       int not null primary key,
    word1_id int not null,
    word2_id int not null,
    word3_id int not null,
    word4_id int not null,

    constraint fk_four_grams_word1_id foreign key (word1_id) references words(id),
    constraint fk_four_grams_word2_id foreign key (word2_id) references words(id),
    constraint fk_four_grams_word3_id foreign key (word3_id) references words(id),
    constraint fk_four_grams_word4_id foreign key (word4_id) references words(id)
);

create unique index idx_four_grams on four_grams(word1_id, word2_id, word3_id, word4_id);

create table four_gram_entries (
    id           int not null primary key,
    four_gram_id  int not null,
    year         int not null,
    match_count  int not null,
    volume_count int not null,

    constraint fk_four_gram_entries_four_gram_id foreign key (four_gram_id) references four_grams(id)
);

create unique index idx_four_gram_entries on four_gram_entries(four_gram_id, year);

create table five_grams (
    id       int not null primary key,
    word1_id int not null,
    word2_id int not null,
    word3_id int not null,
    word4_id int not null,
    word5_id int not null,

    constraint fk_five_grams_word1_id foreign key (word1_id) references words(id),
    constraint fk_five_grams_word2_id foreign key (word2_id) references words(id),
    constraint fk_five_grams_word3_id foreign key (word3_id) references words(id),
    constraint fk_five_grams_word4_id foreign key (word4_id) references words(id),
    constraint fk_five_grams_word5_id foreign key (word5_id) references words(id)
);

create unique index idx_five_grams on five_grams(word1_id, word2_id, word3_id, word4_id, word5_id);

create table five_gram_entries (
    id           int not null primary key,
    five_gram_id  int not null,
    year         int not null,
    match_count  int not null,
    volume_count int not null,

    constraint fk_five_gram_entries_five_gram_id foreign key (five_gram_id) references five_grams(id)
);

create unique index idx_five_gram_entries on five_gram_entries(five_gram_id, year);

create table fetched_files (
    n   int not null,
    idx int not null,

    primary key(n, idx)
);