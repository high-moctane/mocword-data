create table three_gram_scores (
    prefix_id bigint not null,
    suffix_id bigint not null,
    score     bigint not null,

    primary key (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4';
