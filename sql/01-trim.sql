select datetime("now","localtime"), "start";

-- pragma journal_mode = WAL;
pragma journal_mode = OFF;
pragma synchronous = OFF;
pragma mmap_size = 30000000000;
pragma cache_size = -1000000;
pragma locking_mode = EXCLUSIVE;
pragma threads = 4;
pragma soft_heap_limit = 7000000000;



-- ##############

select datetime("now","localtime"), "delete one";

delete
from
    one_gram_records
where
    word regexp "[0-9]+"
    or
    score < 10000
;


select datetime("now","localtime"), "delete two";

delete
from
    two_gram_records
where
    score < 10000
;


select datetime("now","localtime"), "delete three";

delete
from
    three_gram_records
where
    score < 10000
;


select datetime("now","localtime"), "delete four";
delete
from
    four_gram_records
where
    score < 10000
;


select datetime("now","localtime"), "delete five";
delete
from
    five_gram_records
where
    score < 10000
;


-- ##############

select datetime("now","localtime"), "vacuum";
vacuum;

pragma optimize;
select datetime("now","localtime"), "done";
