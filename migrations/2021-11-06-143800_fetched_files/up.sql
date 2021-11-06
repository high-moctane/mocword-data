create table fetched_files (
    n   bigint not null,
    idx bigint not null,

    primary key (n, idx)
) engine innodb charset 'utf8mb4';
