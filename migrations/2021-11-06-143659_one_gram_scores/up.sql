create table one_gram_scores (
    word  text   not null,
    score bigint not null,

    primary key (word(255))
) engine innodb charset 'utf8mb4';
