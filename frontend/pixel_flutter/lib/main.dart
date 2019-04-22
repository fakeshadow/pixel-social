import 'package:flutter/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/Views/HomePage.dart';
import 'package:pixel_flutter/Views/ProfilePage.dart';

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

  @override
  void initState() {
    userBloc = UserBloc();
    errorBloc = ErrorBloc();
    userBloc.dispatch(UserInit());
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocProviderTree(
        blocProviders: [
          BlocProvider<ErrorBloc>(bloc: errorBloc),
          BlocProvider<UserBloc>(bloc: userBloc)
        ],
        child: MaterialApp(
          routes: {
            'profile': (context) => ProfilePage(),
            'home': (context) => HomePage(),
          },
          initialRoute: 'home',
          theme: ThemeData(
              brightness: Brightness.light,
              primarySwatch: Colors.blue,
              accentColor: Colors.deepPurple),
        ));
  }

  @override
  void dispose() {
    userBloc.dispose();
    errorBloc.dispose();
    super.dispose();
  }
}
