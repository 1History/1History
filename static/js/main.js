const SHOW_FORMAT = "YYYY/MM/DD";

function initCharts(dailyVisits, titleTop, domainTop) {
  // console.log(titleTop);
  // console.log(domainTop);
  return function(ec) {
    initDailyVisits(ec, dailyVisits);
    initTop10(ec, titleTop, 'titleTop10', 'TOP10 sites(by title)');
    initTop10(ec, domainTop, 'domainTop10', 'TOP10 sites(by domain)');
  };
}

function initDailyVisits(ec, dailyVisits) {
  var ecConfig = require('echarts/config');
  //--- Trend Chart ---
  var dailyVisitsChart = ec.init(document.getElementById('dailyVisits'));
  dailyVisitsChart.setOption({
    color: ['#23B7E5'],
    title : {
      text : 'Daily PV',
      subtext : 'Click any node to view details'
    },
    tooltip : {
      trigger: 'item',
      formatter : function (params) {
        var date = new Date(params.value[0]);
        data = date.getFullYear() + '/'
          + (date.getMonth() + 1) + '/'
          + date.getDate();
        return data + '<br/>'
          + "PV: " + params.value[1];
      }
    },
    toolbox: {
      show : true,
      feature : {
        mark : {show: true},
        dataView : {show: true, readOnly: false},
        magicType : {show: true, type: ['line', 'bar']},
        restore : {show: true},
        saveAsImage : {show: true}
      }
    },
    dataZoom: {
      show: true,
      start : 0
    },
    legend : {
      data : ['Page View']
    },
    grid: {
      y2: 100
    },
    xAxis : [
      {
        type : 'time',
        splitNumber: 10
      }
    ],
    yAxis : [
      {
        name: 'PV',
        type : 'value'
      }
    ],
    series : [
      {
        name: 'Page View',
        type: 'line',
        showAllSymbol: true,
        symbolSize: function (value){
          return Math.round(value[1]/100) + 2;
        },
        data: (function () {
          return _.map(dailyVisits, function(visit) {
            return [new Date(visit[0]), visit[1]];
          });
        })()
      }
    ]
  });
  dailyVisitsChart.on(ecConfig.EVENT.CLICK, function(param) {
    let url = `/details/${param.data[0].getTime()}`;
    window.open(url, '_blank');
  });

}

function initTop10(ec, topItems, eleId, title) {
  var URLsPercentChart = ec.init(document.getElementById(eleId));
  var topLimit = topItems.length < 10 ? topItems.length : 10;
  var top10DataSource = [];
  var top10Titles = [];
  for (var i = 0; i < topLimit; i++) {
    var head = topItems[i][0];
    head = head.length > 50 ? head.substring(0, 50) : head;
    top10Titles.push(head);
    top10DataSource.push({value: topItems[i][1], name: head});
  }
  URLsPercentChart.setOption({
    title : {
      text: title,
      x:'center'
    },
    tooltip : {
      trigger: 'item',
      formatter: "{a} <br/>{b} : {c} ({d}%)"
    },
    legend: {
      orient : 'vertical',
      x : 'left',
      data: top10Titles
    },
    toolbox: {
      show : true,
      feature : {
        mark : {show: true},
        dataView : {show: true, readOnly: false},
        magicType : {
          show: true,
          type: ['pie', 'funnel'],
          option: {
            funnel: {
              x: '25%',
              width: '50%',
              funnelAlign: 'left',
              max: 1548
            }
          }
        },
        restore : {show: true},
        saveAsImage : {show: true}
      }
    },
    calculable : true,
    series : [
      {
        name:'Title: ',
        type:'pie',
        radius : '75%',
        center: ['50%', '60%'],
        data: top10DataSource
      }
    ]
  });
}

function configChart(dailyVisits, titleTop100, domainTop100) {
  require.config({
    paths: {
      echarts: '/static/js'
    }
  });
  require(
    [
      'echarts',
      'echarts/chart/line',
      'echarts/chart/pie',
      'echarts/chart/bar',
      'echarts/chart/funnel'
    ],
    initCharts(dailyVisits, titleTop100, domainTop100)
  );
}

function chooseDaterangeCB(start, end) {
  $('#browse_range span').html(`${start.format(SHOW_FORMAT)} - ${end.format(SHOW_FORMAT)}`);
}

function ohsearch() {
  let kw = $('#keyword').val();
  let range = $('#browse_range').data('daterangepicker');

  window.location = `/?start=${range.startDate.valueOf()}&end=${range.endDate.valueOf()}&keyword=${kw}`;
}
