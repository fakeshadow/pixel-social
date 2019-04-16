import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter/Views/TopicsPage.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';
import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/NavigationBar/HorizontalTab/HorizontalTabBar.dart';

import 'package:pixel_flutter/components/NavigationBar/TabNavBar.dart';

class HomePage extends StatefulWidget {
  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  @override
  Widget build(BuildContext context) {
    final userBloc = BlocProvider.of<UserBloc>(context);
    final errorBloc = BlocProvider.of<ErrorBloc>(context);
    return Scaffold(
        bottomNavigationBar: TabNavBar(1),
        endDrawer: Container(
          child: Center(child: Text('abcdefg')),
        ),
        body: BlocListener(
            bloc: errorBloc,
            listener: (BuildContext context, ErrorState state) {
              if (state is ShowError) {
                Scaffold.of(context).showSnackBar(SnackBar(
                  backgroundColor: Colors.deepOrangeAccent,
                  content: Text(state.error),
                ));
              }
            },
            child: BlocBuilder(
                bloc: userBloc,
                builder: (BuildContext context, UserState state) {
                  return Scaffold(
                    body: Stack(
                      children: <Widget>[
                        GeneralBackground(),
                        Center(
                          child: HorizontalTab(),
                        )
                      ],
                    )
                  );
//                  if (state is AppStarted) {
//                    userBloc.dispatch(UserInit());
//                    return Center(
//                        child: Container(child: CircularProgressIndicator()));
//                  }
//                  if (state is UserLoaded) {
//                    return TopicsPage();
//                  }
//                  if (state is Loading) {
//                    return Center(
//                        child: Container(child: CircularProgressIndicator()));
//                  }
//                  if (state is UserLoggedOut) {
//                    return AuthenticationPage(
//                      type: 'login',
//                      username: state.username,
//                    );
//                  }
//                  if (state is UserNone) {
//                    return AuthenticationPage(type: 'register');
//                  }
//                  if (state is Failure) {
//                    errorBloc.dispatch(GetError(error: state.error));
//                  }
                  return Center(
                      child: Container(child: CircularProgressIndicator()));
                })));
  }
}
