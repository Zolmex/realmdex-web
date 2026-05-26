use worker::*;

pub async fn run(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;

    db.prepare(
        "INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
         SELECT server_id,
                date(time) AS day,
                COUNT(*) AS total_checks,
                SUM(online) AS up_checks,
                MAX(players) AS peak_players
         FROM server_polls
         WHERE date(time) = date('now', '-1 day')
         GROUP BY server_id, date(time)
         ON CONFLICT(server_id, day) DO UPDATE SET
           total_checks = excluded.total_checks,
           up_checks    = excluded.up_checks,
           peak_players = excluded.peak_players"
    ).run().await?;

    // only delete old polls whose day was already rolled up, so a rollup outage won't lose data
    db.prepare(
        "DELETE FROM server_polls
         WHERE time < datetime('now', '-30 days')
           AND EXISTS (
             SELECT 1 FROM server_polls_daily
             WHERE server_polls_daily.server_id = server_polls.server_id
               AND server_polls_daily.day = date(server_polls.time)
           )"
    ).run().await?;

    Ok(())
}
