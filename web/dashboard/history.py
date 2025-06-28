from flask import Blueprint, render_template

from dashboard.db import get_db

bp = Blueprint("history", __name__)

@bp.route('/history', methods=('GET', 'POST'))
def history():
    db = get_db()
    history = db.execute( 'SELECT * FROM sensor_readings').fetchall()

    return render_template('history.html', history=history)
