select datetime("now","localtime"), "start";

attach database "data.sqlite" as data;

pragma journal_mode = WAL;
pragma synchronous = OFF;
pragma mmap_size = 30000000000;
pragma cache_size = -1000000;
pragma locking_mode = EXCLUSIVE;
pragma threads = 8;
pragma soft_heap_limit = 7000000000;



-- ##############


select datetime("now","localtime"), "idx_one_gram_records_word";
create index if not exists data.idx_one_gram_records_word on one_gram_records (word);

select datetime("now","localtime"), "idx_one_gram_records_score";
create index if not exists data.idx_one_gram_records_score on one_gram_records (score);

select datetime("now","localtime"), "idx_two_gram_records_score";
create index if not exists data.idx_two_gram_records_score on two_gram_records (score);

select datetime("now","localtime"), "idx_three_gram_records_score";
create index if not exists data.idx_three_gram_records_score on three_gram_records (score);

select datetime("now","localtime"), "idx_four_gram_records_score";
create index if not exists data.idx_four_gram_records_score on four_gram_records (score);

select datetime("now","localtime"), "idx_five_gram_records_score";
create index if not exists data.idx_five_gram_records_score on five_gram_records (score);


-- ##############


select datetime("now","localtime"), "analyze";
analyze;


-- ##############


select datetime("now","localtime"), "create table one_grams";
create table one_grams (
    id      integer not null primary key,
    word    text    not null
);

select datetime("now","localtime"), "insert into one_grams";
insert into
    one_grams (word)
select
    word
from
    one_gram_records
where
    word regexp "^[^0-9]+$"
order by score desc, word
;

select datetime("now","localtime"), "create index one_grams";
create index idx_one_grams_word on one_grams (word);

select datetime("now","localtime"), "analyze main";
analyze main;


-- ##############


select datetime("now","localtime"), "create table two_grams";
create table two_grams (
    id      integer not null primary key,
    prefix  integer not null,
    suffix  integer not null,
    foreign key (prefix) references one_grams (id)
        on delete cascade deferrable initially deferred,
    foreign key (suffix) references one_grams (id)
        on delete cascade deferrable initially deferred
);

select datetime("now","localtime"), "insert into two_grams";
insert into
    two_grams (prefix, suffix)
select
    newone1.id, newone2.id
from
    two_gram_records as r
join one_gram_records as oldone1
    on oldone1.id = r.word1
join one_gram_records as oldone2
    on oldone2.id = r.word2
join one_grams as newone1
    on newone1.word = oldone1.word
join one_grams as newone2
    on newone2.word = oldone2.word
order by r.score desc
;

select datetime("now","localtime"), "create index two_grams suffix";
create index idx_two_grams_suffix on two_grams (suffix);

select datetime("now","localtime"), "create index two_grams";
create unique index idx_two_grams_words on two_grams (prefix, suffix);


select datetime("now","localtime"), "analyze main";
analyze main;


-- ##############


select datetime("now","localtime"), "create table three_grams";
create table three_grams (
    id      integer not null primary key,
    prefix  integer not null,
    suffix  integer not null,
    foreign key (prefix) references two_grams (id)
        on delete cascade deferrable initially deferred,
    foreign key (suffix) references one_grams (id)
        on delete cascade deferrable initially deferred
);

select datetime("now","localtime"), "insert into three_grams";
insert into
    three_grams (prefix, suffix)
select
    two.id, newone3.id
from
    three_gram_records as r
join one_gram_records as oldone1
    on oldone1.id = r.word1
join one_gram_records as oldone2
    on oldone2.id = r.word2
join one_gram_records as oldone3
    on oldone3.id = r.word3
join one_grams as newone1
    on newone1.word = oldone1.word
join one_grams as newone2
    on newone2.word = oldone2.word
join one_grams as newone3
    on newone3.word = oldone3.word
join two_grams as two
    on two.prefix = newone1.id and two.suffix = newone2.id
order by r.score desc
;

select datetime("now","localtime"), "create index three_grams suffix";
create index idx_three_grams_suffix on three_grams (suffix);

select datetime("now","localtime"), "create index three_grams";
create unique index idx_three_grams_words on three_grams (prefix, suffix);


select datetime("now","localtime"), "analyze main";
analyze main;


-- ##############


select datetime("now","localtime"), "create table four_grams";
create table four_grams (
    id      integer not null primary key,
    prefix  integer not null,
    suffix  integer not null,
    foreign key (prefix) references three_grams (id)
        on delete cascade deferrable initially deferred,
    foreign key (suffix) references one_grams (id)
        on delete cascade deferrable initially deferred
);

select datetime("now","localtime"), "insert into four_grams";
insert into
    four_grams (prefix, suffix)
select
    three.id, newone4.id
from
    four_gram_records as r
join one_gram_records as oldone1
    on oldone1.id = r.word1
join one_gram_records as oldone2
    on oldone2.id = r.word2
join one_gram_records as oldone3
    on oldone3.id = r.word3
join one_gram_records as oldone4
    on oldone4.id = r.word4
join one_grams as newone1
    on newone1.word = oldone1.word
join one_grams as newone2
    on newone2.word = oldone2.word
join one_grams as newone3
    on newone3.word = oldone3.word
join one_grams as newone4
    on newone4.word = oldone4.word
join two_grams as two
    on two.prefix = newone1.id and two.suffix = newone2.id
join three_grams as three
    on three.prefix = two.id and three.suffix = newone3.id
order by r.score desc
;

select datetime("now","localtime"), "create index four_grams suffix";
create index idx_four_grams_suffix on four_grams (suffix);

select datetime("now","localtime"), "create index four_grams";
create unique index idx_four_grams_words on four_grams (prefix, suffix);


select datetime("now","localtime"), "analyze main";
analyze main;


-- ##############


select datetime("now","localtime"), "create table five_grams";
create table five_grams (
    id      integer not null primary key,
    prefix  integer not null,
    suffix  integer not null,
    foreign key (prefix) references four_grams (id)
        on delete cascade deferrable initially deferred,
    foreign key (suffix) references one_grams (id)
        on delete cascade deferrable initially deferred
);

select datetime("now","localtime"), "insert into five_grams";
insert into
    five_grams (prefix, suffix)
select
    four.id, newone5.id
from
    five_gram_records as r
join one_gram_records as oldone1
    on oldone1.id = r.word1
join one_gram_records as oldone2
    on oldone2.id = r.word2
join one_gram_records as oldone3
    on oldone3.id = r.word3
join one_gram_records as oldone4
    on oldone4.id = r.word4
join one_gram_records as oldone5
    on oldone5.id = r.word5
join one_grams as newone1
    on newone1.word = oldone1.word
join one_grams as newone2
    on newone2.word = oldone2.word
join one_grams as newone3
    on newone3.word = oldone3.word
join one_grams as newone4
    on newone4.word = oldone4.word
join one_grams as newone5
    on newone5.word = oldone5.word
join two_grams as two
    on two.prefix = newone1.id and two.suffix = newone2.id
join three_grams as three
    on three.prefix = two.id and three.suffix = newone3.id
join four_grams as four
    on four.prefix = three.id and four.suffix = newone4.id
order by r.score desc
;

select datetime("now","localtime"), "create index five_grams suffix";
create index idx_five_grams_suffix on five_grams (suffix);

select datetime("now","localtime"), "create index five_grams";
create unique index idx_five_grams_words on five_grams (prefix, suffix);


select datetime("now","localtime"), "analyze main";
analyze main;


-- ##############


select datetime("now","localtime"), "vacuum";
vacuum main;

pragma optimize;
select datetime("now","localtime"), "done";
