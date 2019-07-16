import 'dart:math';

import 'package:flutter/material.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryEvent.dart';

import 'package:sqflite/sqlite_api.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';
import 'package:pixel_flutter/blocs/VerticalTabBloc/VerticalTabBloc.dart';

import 'package:pixel_flutter/Views/HomePage.dart';
import 'package:pixel_flutter/Views/ProfilePage.dart';

import 'package:pixel_flutter/api/DataBase.dart';

void main() {
  runApp(PixelShare());
}

class PixelShare extends StatefulWidget {
  @override
  _PixelShareState createState() => _PixelShareState();
}

class _PixelShareState extends State<PixelShare> {
  UserBloc userBloc;
  ErrorBloc errorBloc;
  TalkBloc talkBloc;
  MessageBloc messageBloc;
  CategoryBloc categoryBloc;
  Database db;

  @override
  void initState() {
    init();
    super.initState();
  }

  Future<void> init() async {
//    await DataBase.delDb();
    await DataBase.createDb();
    final d = await DataBase.getDb();
    setState(() {
      db = d;
      errorBloc = ErrorBloc();
      categoryBloc = CategoryBloc(errorBloc: errorBloc, db: d);
      messageBloc = MessageBloc(errorBloc: errorBloc);
      talkBloc =
          TalkBloc(messageBloc: messageBloc, errorBloc: errorBloc, db: d);
      userBloc = UserBloc(talkBloc: talkBloc, db: d);
    });

    userBloc.dispatch(UserInit());
    categoryBloc.dispatch(LoadCategories());
  }

  @override
  void dispose() {
    userBloc.dispose();
    errorBloc.dispose();
    talkBloc.dispose();
    db.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (db == null) {
      //ToDo: add loading page
      return Container();
    } else {
      return MultiBlocProvider(
        providers: [
          // bloc for handling error info
          BlocProvider<ErrorBloc>(builder: (context) => errorBloc),
          // bloc for handling user data
          BlocProvider<UserBloc>(builder: (context) => userBloc),
          // bloc for handling talks
          BlocProvider<TalkBloc>(builder: (context) => talkBloc),
          // bloc for handling categories data
          BlocProvider<CategoryBloc>(builder: (context) => categoryBloc),
          // bloc for handling vertical tab bar
          BlocProvider<VerticalTabBloc>(builder: (context) => VerticalTabBloc())
        ],
        child: MaterialApp(
          routes: {
            'profile': (context) => ProfilePage(),
            'home': (context) => HomePage(),
          },
          debugShowCheckedModeBanner: false,
          initialRoute: 'home',
          theme: ThemeData(
              brightness: Brightness.light,
              primarySwatch: Colors.blue,
              accentColor: Colors.deepPurple),
        ),
      );
    }
  }
}
