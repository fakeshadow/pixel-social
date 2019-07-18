import 'package:flutter/material.dart';

import 'package:sqflite/sqlite_api.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryBloc.dart';
import 'package:pixel_flutter/blocs/CategoryBloc/CategoryEvent.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/MessageBloc/MessageBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';
import 'package:pixel_flutter/blocs/UsersBloc/UsersBloc.dart';
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
  UsersBloc usersBloc;
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
      // users bloc handle other users data
      usersBloc = UsersBloc(errorBloc: errorBloc, db: d);
      messageBloc = MessageBloc(errorBloc: errorBloc);
      categoryBloc = CategoryBloc(errorBloc: errorBloc, db: d);
      // talk bloc handle talks list and listen to websocket and dispatch messages to other blocs.
      talkBloc = TalkBloc(
          errorBloc: errorBloc,
          messageBloc: messageBloc,
          usersBloc: usersBloc,
          db: d);
      // user bloc handle self user data, authentication and handle the connection of websocket.
      userBloc = UserBloc(talkBloc: talkBloc, db: d);

      // load user event will dispatch to talk bloc and load all talks,friends and unread messages.
      userBloc.dispatch(LoadUser());
      categoryBloc.dispatch(LoadCategories());
    });
  }

  @override
  void dispose() {
    db.close();
    errorBloc.dispose();
    usersBloc.dispose();
    messageBloc.dispose();
    categoryBloc.dispose();
    talkBloc.dispose();
    userBloc.dispose();
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
          BlocProvider<ErrorBloc>(builder: (context) => errorBloc),
          BlocProvider<UserBloc>(builder: (context) => userBloc),
          BlocProvider<CategoryBloc>(builder: (context) => categoryBloc),
          BlocProvider<TalkBloc>(builder: (context) => talkBloc),
          BlocProvider<UsersBloc>(builder: (context) => usersBloc),
          BlocProvider<MessageBloc>(builder: (context) => messageBloc),
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
