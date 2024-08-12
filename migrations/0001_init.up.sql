-- Role: admin, customer, sales, driver
-- WhType: counter, warehouse, distcenter
-- Status: driver, warehouse, completed

-- order_status -> Temporary Record

-- tracings.status -> tracings.subject:
-- > warehouse  -> sales
-- > driver     -> driver
-- > completed  -> sales

-- tracings.status -> tracings.wh:
-- > warehouse  -> current
-- > driver     -> destination wh
-- > completed  -> wh where this completed to ?


create table users (
  user_id         int generated always as identity primary key,
  name            text not null unique,
  phone           text not null unique,
  password        text not null,
  role            text not null,
  metadata        json not null default '{}'::json,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now(),
  verified_at     timestamptz
);

create table warehouses (
  wh_id           int generated always as identity primary key,
  wh_name         text not null,
  wh_type         text not null,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now()
);

create table employees (
  user_id         int not null references users(user_id),
  wh_id           int not null references warehouses(wh_id),
  created_at      timestamptz not null default now(),
  primary key     (user_id, wh_id)
);

create table users_snapshot (
  snapshot_id     int generated always as identity primary key,
  data            json not null, -- Users
  snapshoted_at   timestamptz not null default now()
);

create table wh_snapshot (
  snapshot_id     int generated always as identity primary key,
  data            json not null, -- Warehouses
  snapshoted_at   timestamptz not null default now()
);

create table orders (
  order_id        int generated always as identity primary key,
  sender_sid      int not null references users_snapshot(snapshot_id),
  receiver_sid    int not null references users_snapshot(snapshot_id),
  destination     text not null,
  packages        text not null -- { name, weight, length, width, height }[]
);

create table tracings (
  tracing_id      int generated always as identity primary key,
  order_id        int not null references orders(order_id),
  subject_sid     int not null references users_snapshot(snapshot_id),
  wh_sid          int not null references wh_snapshot(snapshot_id),
  status          text not null,
  traced_at       timestamptz not null default now()
);

create table order_status (
  order_id        int not null references orders(order_id),
  tracing_id      int not null references tracings(tracing_id),
  wh_id           int not null references warehouses(wh_id),
  primary key     (order_id, tracing_id)
);

create table manifests (
  manifest_id     int generated always as identity primary key,
  sales_sid       int not null references users_snapshot(snapshot_id),
  driver_sid      int not null references users_snapshot(snapshot_id),
  wh_from_sid     int not null references wh_snapshot(snapshot_id),
  wh_to_sid       int not null references wh_snapshot(snapshot_id),
  created_at      timestamptz not null default now(),
  completed_at    timestamptz
);

create table manifest_orders (
  manifest_id     int not null references manifests(manifest_id) primary key,
  order_id        int not null references orders(order_id)
);


