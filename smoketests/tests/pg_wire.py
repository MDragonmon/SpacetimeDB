from .. import Smoketest
import subprocess
import os
import tomllib


def psql(identity: str, sql: str) -> str:
    """Call `psql` and execute the given SQL statement."""

    result = subprocess.run(
        ["psql", "port=5432 host=127.0.0.1 user=postgres dbname=quickstart", "--quiet", "-c", sql],
        encoding="utf8",
        env={**os.environ, "PGPASSWORD": identity},
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        # check=True
    )

    if result.stderr:
        raise Exception(result.stderr.strip())
    return result.stdout.strip()


class SqlFormat(Smoketest):
    AUTOPUBLISH = False
    MODULE_CODE = """
use spacetimedb::sats::{i256, u256};
use spacetimedb::{ConnectionId, Identity, ReducerContext, Table, Timestamp, TimeDuration};

#[derive(Copy, Clone)]
#[spacetimedb::table(name = t_ints, public)]
pub struct TInts {
    i8: i8,
    i16: i16,
    i32: i32,
    i64: i64,
    i128: i128,
    i256: i256,
}

#[spacetimedb::table(name = t_ints_tuple, public)]
pub struct TIntsTuple {
    tuple: TInts,
}

#[derive(Copy, Clone)]
#[spacetimedb::table(name = t_uints, public)]
pub struct TUints {
    u8: u8,
    u16: u16,
    u32: u32,
    u64: u64,
    u128: u128,
    u256: u256,
}

#[spacetimedb::table(name = t_uints_tuple, public)]
pub struct TUintsTuple {
    tuple: TUints,
}

#[derive(Clone)]
#[spacetimedb::table(name = t_others, public)]
pub struct TOthers {
    bool: bool,
    f32: f32,
    f64: f64,
    str: String,
    bytes: Vec<u8>,
    identity: Identity,
    connection_id: ConnectionId,
    timestamp: Timestamp,
    duration:  TimeDuration,
}

#[spacetimedb::table(name = t_others_tuple, public)]
pub struct TOthersTuple {
    tuple: TOthers
}

#[spacetimedb::reducer]
pub fn test(ctx: &ReducerContext) {
    let tuple = TInts {
        i8: -25,
        i16: -3224,
        i32: -23443,
        i64: -2344353,
        i128: -234434897853,
        i256: (-234434897853i128).into(),
    };
    ctx.db.t_ints().insert(tuple);
    ctx.db.t_ints_tuple().insert(TIntsTuple { tuple });

    let tuple = TUints {
        u8: 105,
        u16: 1050,
        u32: 83892,
        u64: 48937498,
        u128: 4378528978889,
        u256: 4378528978889u128.into(),
    };
    ctx.db.t_uints().insert(tuple);
    ctx.db.t_uints_tuple().insert(TUintsTuple { tuple });

    let tuple = TOthers {
        bool: true,
        f32: 594806.58906,
        f64: -3454353.345389043278459,
        str: "This is spacetimedb".to_string(),
        bytes: vec!(1, 2, 3, 4, 5, 6, 7),
        identity: Identity::ONE,
        connection_id: ConnectionId::ZERO,      
        timestamp: Timestamp::UNIX_EPOCH,
        duration: TimeDuration::from_micros(1000 * 10000),
    };
    ctx.db.t_others().insert(tuple.clone());
    ctx.db.t_others_tuple().insert(TOthersTuple { tuple });
}
"""

    def assertSql(self, token: str, sql: str, expected):
        self.maxDiff = None
        sql_out = psql(token, sql)
        sql_out = "\n".join([line.rstrip() for line in sql_out.splitlines()])
        expected = "\n".join([line.rstrip() for line in expected.splitlines()])
        print(sql_out)
        self.assertMultiLineEqual(sql_out, expected)

    def test_sql_format(self):
        """This test is designed to test calling `psql` to execute SQL statements"""
        with open(self.config_path, "rb") as f:
            config = tomllib.load(f)
            token = config['spacetimedb_token']
        self.publish_module("quickstart", clear=False)

        self.call("test")

        self.assertSql(token, "SELECT * FROM t_ints", """\
i8  |  i16  |  i32   |   i64    |     i128      |     i256
-----+-------+--------+----------+---------------+---------------
 -25 | -3224 | -23443 | -2344353 | -234434897853 | -234434897853
(1 row)""")
        self.assertSql(token, "SELECT * FROM t_ints_tuple", """\
tuple
---------------------------------------------------------------------------------------------------------
 {"i8": -25, "i16": -3224, "i32": -23443, "i64": -2344353, "i128": -234434897853, "i256": -234434897853}
(1 row)""")
        self.assertSql(token, "SELECT * FROM t_uints", """\
u8  | u16  |  u32  |   u64    |     u128      |     u256
-----+------+-------+----------+---------------+---------------
 105 | 1050 | 83892 | 48937498 | 4378528978889 | 4378528978889
(1 row)""")
        self.assertSql(token, "SELECT * FROM t_uints_tuple", """\
tuple
-------------------------------------------------------------------------------------------------------
 {"u8": 105, "u16": 1050, "u32": 83892, "u64": 48937498, "u128": 4378528978889, "u256": 4378528978889}
(1 row)""")
        self.assertSql(token, "SELECT * FROM t_others", """\
bool |    f32    |         f64         |         str         |      bytes       |                              identity                              |           connection_id            |         timestamp         | duration
------+-----------+---------------------+---------------------+------------------+--------------------------------------------------------------------+------------------------------------+---------------------------+----------
 t    | 594806.56 | -3454353.3453890434 | This is spacetimedb | \\x01020304050607 | \\x0000000000000000000000000000000000000000000000000000000000000001 | \\x00000000000000000000000000000000 | 1970-01-01T00:00:00+00:00 | PT10S
(1 row)""")
        self.assertSql(token, "SELECT * FROM t_others_tuple", """\
tuple
-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
 {"bool": true, "f32": 594806.56, "f64": -3454353.3453890434, "str": "This is spacetimedb", "bytes": 0x01020304050607, "identity": 0x0000000000000000000000000000000000000000000000000000000000000001, "connection_id": 0x00000000000000000000000000000000, "timestamp": 1970-01-01T00:00:00+00:00, "duration": PT10S}
(1 row)""")
