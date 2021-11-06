create table four_grams (
    id        bigint not null auto_increment primary key,
    prefix_id bigint not null,
    suffix_id bigint not null,

    constraint ibfk_four_gram_prefix_id
       foreign key ibfk_prefix_id (prefix_id)
       references three_grams (id)
       on delete cascade,
    constraint ibfk_four_gram_suffix_id
       foreign key ibfk_suffix_id (suffix_id)
       references one_grams (id)
       on delete cascade,
    unique index idx_four_grams (prefix_id, suffix_id)
) engine innodb charset 'utf8mb4';
