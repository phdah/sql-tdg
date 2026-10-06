select
    id as order_id,
    customer_id,
    amount,
    status,
    created_at,
    region,
    case
        when amount >= 40 then 'medium'
        else 'low'
    end as amount_bucket,
    case
        when status = 'paid' then 1
        else 0
    end as paid_flag
from {{ source('raw', 'orders') }}
where amount between 10 and 100
  and status = 'paid'
  and region = 'north'
