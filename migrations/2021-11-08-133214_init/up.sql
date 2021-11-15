create table if not exists one_gram_scores (
    id    bigint not null auto_increment primary key,
    word  text   not null,
    score bigint not null
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists two_gram_scores (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists three_gram_scores (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists four_gram_scores (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists five_gram_scores (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists one_grams (
    id    bigint not null auto_increment primary key,
    word  text   not null,
    score bigint not null,

    index idx_one_grams_word (word(255)),
    index idx_one_grams_score (score)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists two_grams (
    id        bigint not null auto_increment primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    constraint ibfk_two_gram_prefix_id
       foreign key ibfk_prefix_id (prefix_id)
       references one_grams (id)
       on delete cascade,
    constraint ibfk_two_gram_suffix_id
       foreign key ibfk_suffix_id (suffix_id)
       references one_grams (id)
       on delete cascade,

    unique index idx_two_grams (prefix_id, suffix_id),
    index idx_two_grams_score (score)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists three_grams (
    id        bigint not null auto_increment primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    constraint ibfk_three_gram_prefix_id
       foreign key ibfk_prefix_id (prefix_id)
       references two_grams (id)
       on delete cascade,
    constraint ibfk_three_gram_suffix_id
       foreign key ibfk_suffix_id (suffix_id)
       references one_grams (id)
       on delete cascade,

    unique index idx_three_grams (prefix_id, suffix_id),
    index idx_three_grams_score (score)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists four_grams (
    id        bigint not null auto_increment primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    constraint ibfk_four_gram_prefix_id
       foreign key ibfk_prefix_id (prefix_id)
       references three_grams (id)
       on delete cascade,
    constraint ibfk_four_gram_suffix_id
       foreign key ibfk_suffix_id (suffix_id)
       references one_grams (id)
       on delete cascade,

    unique index idx_four_grams (prefix_id, suffix_id),
    index idx_four_grams_score (score)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists five_grams (
    id        bigint not null auto_increment primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    constraint ibfk_five_gram_prefix_id
       foreign key ibfk_prefix_id (prefix_id)
       references four_grams (id)
       on delete cascade,
    constraint ibfk_five_gram_suffix_id
       foreign key ibfk_suffix_id (suffix_id)
       references one_grams (id)
       on delete cascade,

    unique index idx_five_grams (prefix_id, suffix_id),
    index idx_five_grams_score (score)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';

create table if not exists fetched_files (
    n   bigint not null,
    idx bigint not null,

    primary key (n, idx)
) engine innodb charset 'utf8mb4' collate 'utf8mb4_bin';
