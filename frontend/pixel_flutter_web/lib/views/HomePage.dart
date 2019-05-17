import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';

import 'package:pixel_flutter_web/components/BasicLayout.dart';
import 'package:pixel_flutter_web/components/BasicSliverPadding.dart';
import 'package:pixel_flutter_web/components/TopicsList.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';
import 'package:pixel_flutter_web/components/FloatingAppBar.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';

class HomePage extends StatefulWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  TopicsBloc _topicsBloc;
  final scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<CategoryBloc>(context).dispatch(LoadCategories());
    _topicsBloc = TopicsBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: 1));
    scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
    scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return WillPopScope(
        onWillPop: onWillPop,
        child: BasicLayout(
          scrollView: scrollView(),
          sideMenu: SideMenu(),
        ));
  }

  Widget scrollView() {
    return Scrollbar(
      child: CustomScrollView(
        controller: scrollController,
        slivers: [
          FloatingAppBar(title: 'pixelshare example'),
          BasicSliverPadding(sliver: TopicsList(topicsBloc: _topicsBloc))
        ],
      ),
    );
  }

  void _onScroll() {
    final maxScroll = scrollController.position.maxScrollExtent;
    final currentScroll = scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicsBloc.dispatch(GetTopics(categoryId: 1));
    }
  }

  Future<bool> onWillPop() {
    return showDialog(
        context: context,
        builder: (context) =>
            AlertDialog(
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
}
