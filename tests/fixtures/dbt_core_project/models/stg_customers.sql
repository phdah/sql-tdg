select
    id as customer_id,
    score,
    active
from {{ source('raw', 'customers') }}
where score = 7
  and active = true
