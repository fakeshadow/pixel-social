import 'package:path/path.dart';
import 'package:sqflite/sqflite.dart';

import 'package:pixel_flutter/models/Talk.dart';

const String talkTable = 'CREATE TABLE talks ('
    'id INTEGER PRIMARY KEY,'
    'name TEXT,'
    'description TEXT,'
    'privacy INTEGER,'
    'owner INTEGER,'
    'admin BLOB,'
    'users BLOB);';

const String messageTables = 'CREATE TABLE publicMessages ('
    'talk_id INTEGER PRIMARY KEY,'
    'time INTEGER,'
    'message TEXT);'
    'CREATE TABLE privateMessages ('
    'from_id INTEGER PRIMARY KEY,'
    'time INTEGER,'
    'message TEXT);';

const String keyTable = 'CREATE TABLE keys ('
    'key TEXT PRIMARY KEY,'
    'value TEXT)';

const String queryTalks = 'SELECT * FROM talks';

class DataBase {
  static Future<String> pathGen() async {
    final databasePath = await getDatabasesPath();
    return join(databasePath, 'pixeshare.db');
  }

  static Future<void> delDb() async {
    final path = await pathGen();
    await deleteDatabase(path);
  }

  static Future<void> createDb() async {
    final path = await pathGen();
    final exists = await databaseExists(path);
    if (!exists) {
      print("creating database");
      final Database db = await openDatabase(path, version: 1,
          onCreate: (Database db, int version) async {
        await db.execute(talkTable);
        await db.execute(messageTables);
        await db.execute(keyTable);
      });
      db.close();
    }
  }

  static Future<List<Talk>> getTalks() async {
    final path = await pathGen();
    final Database db = await openReadOnlyDatabase(path);
    List<Map> list = await db.rawQuery(queryTalks);
    final List<Talk> talks = list.map((col) {
      return Talk(
          id: col['id'],
          name: col['name'],
          description: col['description'],
          privacy: col['privacy'],
          owner: col['owner'],
          admin: col['admin'],
          users: col['users']);
    }).toList();
    db.close();
    return talks;
  }

  static Future<void> saveTalks({List<Talk> talks}) async {
    final path = await pathGen();
    final Database db = await openReadOnlyDatabase(path);
    var batch = db.batch();
    for (var t in talks) {
      batch.insert('talks', t.toMap());
    }
    await batch.commit();
    db.close();
  }
}
