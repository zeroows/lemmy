
alter table private_message alter column ap_id set not null;
alter table private_message alter column ap_id set default 'http://fake.com';

alter table post alter column ap_id set not null;
alter table post alter column ap_id set default 'http://fake.com';

alter table comment alter column ap_id set not null;
alter table comment alter column ap_id set default 'http://fake.com';

update private_message
set ap_id = 'http://fake.com'
where ap_id like 'changeme_%';

update post
set ap_id = 'http://fake.com'
where ap_id like 'changeme_%';

update comment
set ap_id = 'http://fake.com'
where ap_id like 'changeme_%';
