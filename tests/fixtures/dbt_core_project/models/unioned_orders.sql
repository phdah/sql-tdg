select
    amount,
    region
from {{ ref('enriched_orders') }}
union all
select
    amount,
    region
from {{ source('raw', 'legacy_orders') }}
where amount = 42
  and region = 'north'
