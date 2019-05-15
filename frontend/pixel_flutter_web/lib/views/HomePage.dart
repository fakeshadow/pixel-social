import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';

import 'package:pixel_flutter_web/components/UserButton.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/TopicsLayout.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/views/TopicPage.dart';

class HomePage extends StatefulWidget with env {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  TopicsBloc _topicBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<CategoryBloc>(context).dispatch(LoadCategories());
    _topicBloc = TopicsBloc();
    _topicBloc.dispatch(GetTopics(categoryId: 1));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return WillPopScope(
      onWillPop: onWillPop,
      child: BlocBuilder(
          bloc: BlocProvider.of<UserBloc>(context),
          builder: (context, userState) {
            return Scaffold(
              endDrawer: userState is UserLoaded ? UserDrawer() : null,
              persistentFooterButtons: <Widget>[],
              appBar: AppBar(
                primary: false,
                flexibleSpace:
                    FlexibleSpaceBar(title: Text('pixelshare example')),
                elevation: 5.0,
                leading: MediaQuery.of(context).size.width < widget.BREAK_POINT_WIDTH
                    ? IconButton(
                        onPressed: () => Navigator.pushNamed(context, '/home'),
                        icon: Icon(Icons.home),
                      )
                    : Container(),
                actions: <Widget>[UserButton()],
              ),
              body: BlocListener(
                bloc: BlocProvider.of<ErrorBloc>(context),
                listener: (BuildContext context, ErrorState state) async {
                  snackbarController(context, state);
                },
                child: TopicsLayout(
                    topicBloc: _topicBloc, controller: _scrollController),
              ),
            );
          }),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopics(categoryId: 1));
    }
  }

  Future<bool> onWillPop() {
    return showDialog(
        context: context,
        builder: (context) => AlertDialog(
              title: Text('Leaving?'),
              content: Text('Do you want to exit the app'),
              actions: <Widget>[
                FlatButton(
                  onPressed: () => Navigator.pop(context, false),
                  child: Text(
                    'Cancel',
                    style: recoverButtonStyle,
                  ),
                ),
                RaisedButton(
                  color: primaryColor,
                  onPressed: () => Navigator.pop(context, true),
                  child: Text(
                    'Confirm',
                    style: submitButtonStyle.copyWith(fontSize: 16),
                  ),
                )
              ],
            ));
  }

  snackbarController(BuildContext context, ErrorState state) async {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }
}
