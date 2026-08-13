-- Refunds announced on reserves, kept after they are settled so that past refunds stay visible.
-- Not tied to `reserves` by a foreign key on purpose: a reserve row is deleted as soon as its box
-- is spent, and the refund history has to outlive it.
CREATE TABLE refunds (
    id INTEGER PRIMARY KEY NOT NULL,
    reserve_identifier CHAR(32) NOT NULL,
    -- nanoERG announced at initiation (upper bound the contract enforces on withdrawal)
    amount BIGINT NOT NULL,
    -- nanoERG actually withdrawn, set when the refund completes
    withdrawn_amount BIGINT,
    -- height written into R5 of the reserve box, the waiting period counts from here
    init_height INTEGER NOT NULL,
    status TEXT CHECK (status IN ('initiated', 'completed', 'cancelled')) NOT NULL,
    -- transaction that announced the refund, absent for a refund this server did not announce
    -- itself and only found on chain
    init_tx_id CHAR(64),
    settle_tx_id CHAR(64)
);

CREATE INDEX refund_reserve_idx ON refunds(reserve_identifier);

-- The contract allows a single pending refund per reserve, mirror that here so a double
-- initiation is rejected before a transaction is ever built.
CREATE UNIQUE INDEX refund_pending_idx ON refunds(reserve_identifier) WHERE status = 'initiated';
