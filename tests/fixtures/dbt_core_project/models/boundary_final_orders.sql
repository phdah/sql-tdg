select
    amount
from {{ ref('stg_orders') }}
where amount = 42
