import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

import 'package:pixel_flutter/Views/TopicsPage.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';

class HomePage extends StatefulWidget {
  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  @override
  Widget build(BuildContext context) {
    final userBloc = BlocProvider.of<UserBloc>(context);
    return BlocBuilder(
        bloc: userBloc,
        builder: (BuildContext context, UserState state) {
          if (state is AppStarted) {
            userBloc.dispatch(UserInit());
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserLoaded) {
            return TopicsPage();
          }
          if (state is Loading) {
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserLoggedOut) {
            return AuthenticationPage(
              type: 'login',
              username: state.username,
            );
          }
          if (state is UserNone) {
            return AuthenticationPage(type: 'login');
          }
          if (state is Failure) {
            return Container(child: Center(
              child: Text(state.error),
            ));
          }
        });
  }
}
