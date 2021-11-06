create table one_grams (
    id   bigint not null auto_increment primary key,
    word text   not null unique
) engine innodb charset 'utf8mb4';
